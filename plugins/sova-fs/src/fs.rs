//! [`Fs`] handle + [`FsPlugin`] + [`FsExt`].

use crate::events::{DirCreated, FileRemoved, FileWritten};
use crate::path::{rel_display, resolve};
use crate::FsError;
use sova_core::{App, EventBus, Plugin, Request};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct FsEntry {
    pub path: String,
    pub name: String,
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct FsMeta {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
    pub modified: Option<SystemTime>,
}

#[derive(Clone)]
struct FsInner {
    root: PathBuf,
    max_walk_depth: usize,
    max_walk_entries: usize,
    events: Option<EventBus>,
}

/// Cloneable filesystem handle rooted at a jail directory.
#[derive(Clone)]
pub struct Fs {
    inner: Arc<FsInner>,
}

impl Fs {
    /// Builder for [`FsPlugin`] (installed via `app.install`).
    #[allow(clippy::new_ret_no_self)]
    pub fn new(root: impl Into<PathBuf>) -> FsPlugin {
        FsPlugin {
            root: root.into(),
            root_explicit: true,
            max_walk_depth: 32,
            max_walk_entries: 10_000,
        }
    }

    /// Build from `SOVA_FS_ROOT` (default `./data`).
    pub fn from_env() -> FsPlugin {
        let root = std::env::var("SOVA_FS_ROOT").unwrap_or_else(|_| "./data".into());
        FsPlugin {
            root: PathBuf::from(root),
            root_explicit: std::env::var_os("SOVA_FS_ROOT").is_some(),
            max_walk_depth: 32,
            max_walk_entries: 10_000,
        }
    }

    pub fn root(&self) -> &Path {
        &self.inner.root
    }

    fn emit_written(&self, path: &Path) {
        if let Some(bus) = &self.inner.events {
            bus.dispatch(FileWritten {
                path: rel_display(&self.inner.root, path),
            });
        }
    }

    fn emit_removed(&self, path: &Path) {
        if let Some(bus) = &self.inner.events {
            bus.dispatch(FileRemoved {
                path: rel_display(&self.inner.root, path),
            });
        }
    }

    fn emit_dir(&self, path: &Path) {
        if let Some(bus) = &self.inner.events {
            bus.dispatch(DirCreated {
                path: rel_display(&self.inner.root, path),
            });
        }
    }

    async fn resolve(&self, relative: &str) -> Result<PathBuf, FsError> {
        resolve(&self.inner.root, relative).await
    }

    pub async fn exists(&self, path: &str) -> Result<bool, FsError> {
        let p = self.resolve(path).await?;
        Ok(tokio::fs::try_exists(&p).await?)
    }

    pub async fn metadata(&self, path: &str) -> Result<FsMeta, FsError> {
        let p = self.resolve(path).await?;
        let meta = tokio::fs::metadata(&p).await.map_err(map_io)?;
        Ok(meta_to_fs(&meta))
    }

    pub async fn read(&self, path: &str) -> Result<Vec<u8>, FsError> {
        let p = self.resolve(path).await?;
        tokio::fs::read(&p).await.map_err(map_io)
    }

    pub async fn read_to_string(&self, path: &str) -> Result<String, FsError> {
        let p = self.resolve(path).await?;
        tokio::fs::read_to_string(&p).await.map_err(map_io)
    }

    pub async fn write(&self, path: &str, data: impl AsRef<[u8]>) -> Result<(), FsError> {
        let p = self.resolve(path).await?;
        if let Some(parent) = p.parent() {
            if parent != self.inner.root.as_path() && !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        tokio::fs::write(&p, data).await?;
        self.emit_written(&p);
        Ok(())
    }

    pub async fn write_string(&self, path: &str, data: impl AsRef<str>) -> Result<(), FsError> {
        self.write(path, data.as_ref().as_bytes()).await
    }

    pub async fn append(&self, path: &str, data: impl AsRef<[u8]>) -> Result<(), FsError> {
        use tokio::io::AsyncWriteExt;
        let p = self.resolve(path).await?;
        if let Some(parent) = p.parent() {
            if parent != self.inner.root.as_path() && !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
            .await?;
        file.write_all(data.as_ref()).await?;
        self.emit_written(&p);
        Ok(())
    }

    pub async fn create_dir(&self, path: &str) -> Result<(), FsError> {
        let p = self.resolve(path).await?;
        tokio::fs::create_dir_all(&p).await?;
        self.emit_dir(&p);
        Ok(())
    }

    pub async fn remove_file(&self, path: &str) -> Result<(), FsError> {
        let p = self.resolve(path).await?;
        tokio::fs::remove_file(&p).await.map_err(map_io)?;
        self.emit_removed(&p);
        Ok(())
    }

    pub async fn remove_dir(&self, path: &str) -> Result<(), FsError> {
        let p = self.resolve(path).await?;
        if p == self.inner.root {
            return Err(FsError::Forbidden);
        }
        tokio::fs::remove_dir_all(&p).await.map_err(map_io)?;
        self.emit_removed(&p);
        Ok(())
    }

    pub async fn copy(&self, from: &str, to: &str) -> Result<(), FsError> {
        let src = self.resolve(from).await?;
        let dst = self.resolve(to).await?;
        if let Some(parent) = dst.parent() {
            if parent != self.inner.root.as_path() && !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        tokio::fs::copy(&src, &dst).await.map_err(map_io)?;
        self.emit_written(&dst);
        Ok(())
    }

    pub async fn rename(&self, from: &str, to: &str) -> Result<(), FsError> {
        let src = self.resolve(from).await?;
        let dst = self.resolve(to).await?;
        if let Some(parent) = dst.parent() {
            if parent != self.inner.root.as_path() && !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        tokio::fs::rename(&src, &dst).await.map_err(map_io)?;
        self.emit_removed(&src);
        self.emit_written(&dst);
        Ok(())
    }

    pub async fn read_dir(&self, path: &str) -> Result<Vec<FsEntry>, FsError> {
        let p = self.resolve(path).await?;
        let mut rd = tokio::fs::read_dir(&p).await.map_err(map_io)?;
        let mut out = Vec::new();
        while let Some(entry) = rd.next_entry().await.map_err(map_io)? {
            out.push(entry_to_fs(&self.inner.root, &entry).await?);
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub async fn walk(&self, path: &str) -> Result<Vec<FsEntry>, FsError> {
        let p = self.resolve(path).await?;
        let mut out = Vec::new();
        walk_dir(
            &self.inner.root,
            &p,
            0,
            self.inner.max_walk_depth,
            self.inner.max_walk_entries,
            &mut out,
        )
        .await?;
        Ok(out)
    }
}

async fn walk_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    max_entries: usize,
    out: &mut Vec<FsEntry>,
) -> Result<(), FsError> {
    if depth > max_depth {
        return Err(FsError::Msg(format!("walk max_depth={max_depth} exceeded")));
    }
    let mut rd = tokio::fs::read_dir(dir).await.map_err(map_io)?;
    while let Some(entry) = rd.next_entry().await.map_err(map_io)? {
        if out.len() >= max_entries {
            return Err(FsError::Msg(format!(
                "walk max_entries={max_entries} exceeded"
            )));
        }
        let fs_entry = entry_to_fs(root, &entry).await?;
        let is_dir = fs_entry.is_dir;
        let child = entry.path();
        out.push(fs_entry);
        if is_dir {
            Box::pin(walk_dir(
                root,
                &child,
                depth + 1,
                max_depth,
                max_entries,
                out,
            ))
            .await?;
        }
    }
    Ok(())
}

async fn entry_to_fs(root: &Path, entry: &tokio::fs::DirEntry) -> Result<FsEntry, FsError> {
    let path = entry.path();
    let meta = entry.metadata().await.map_err(map_io)?;
    let name = entry.file_name().to_string_lossy().into_owned();
    Ok(FsEntry {
        path: rel_display(root, &path),
        name,
        is_file: meta.is_file(),
        is_dir: meta.is_dir(),
        len: meta.len(),
        modified: meta.modified().ok(),
    })
}

fn meta_to_fs(meta: &std::fs::Metadata) -> FsMeta {
    FsMeta {
        is_file: meta.is_file(),
        is_dir: meta.is_dir(),
        len: meta.len(),
        modified: meta.modified().ok(),
    }
}

fn map_io(e: std::io::Error) -> FsError {
    if e.kind() == std::io::ErrorKind::NotFound {
        FsError::NotFound
    } else {
        FsError::Io(e)
    }
}

/// Plugin builder installed via [`Plugin::install`].
pub struct FsPlugin {
    root: PathBuf,
    root_explicit: bool,
    max_walk_depth: usize,
    max_walk_entries: usize,
}

impl FsPlugin {
    pub fn max_walk_depth(mut self, n: usize) -> Self {
        self.max_walk_depth = n;
        self
    }

    pub fn max_walk_entries(mut self, n: usize) -> Self {
        self.max_walk_entries = n;
        self
    }

    /// Build handle after ensuring root exists and is canonical (for tests).
    pub async fn into_fs(self) -> Result<Fs, FsError> {
        prepare_root(&self.root).await.map(|root| Fs {
            inner: Arc::new(FsInner {
                root,
                max_walk_depth: self.max_walk_depth,
                max_walk_entries: self.max_walk_entries,
                events: None,
            }),
        })
    }
}

async fn prepare_root(root: &Path) -> Result<PathBuf, FsError> {
    tokio::fs::create_dir_all(root).await?;
    Ok(tokio::fs::canonicalize(root).await?)
}

/// `req.fs()`.
pub trait FsExt {
    fn fs(&self) -> Fs;
}

impl FsExt for Request {
    fn fs(&self) -> Fs {
        self.try_state::<Fs>()
            .map(|a| (*a).clone())
            .expect("Fs plugin is not installed (missing req.fs())")
    }
}

impl Plugin for FsPlugin {
    fn id(&self) -> &'static str {
        "fs"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Fs")
            .description("Local filesystem with jail root (async CRUD + walk)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        let mut root = self.root;
        let mut explicit = self.root_explicit;

        if !explicit {
            if let Some(doc) = app.config_doc() {
                if let Some(section) = doc.section("fs") {
                    if let Some(r) = section.get("root").and_then(|v| v.as_str()) {
                        root = PathBuf::from(r);
                        explicit = true;
                        let _ = explicit;
                    }
                }
            }
            if let Ok(env) = std::env::var("SOVA_FS_ROOT") {
                if !env.is_empty() {
                    root = PathBuf::from(env);
                }
            }
        }

        let max_walk_depth = self.max_walk_depth;
        let max_walk_entries = self.max_walk_entries;
        let events = Some(app.events());

        // Sync create + canonicalize so state is ready before accept.
        std::fs::create_dir_all(&root).unwrap_or_else(|e| {
            panic!("sova-fs: create root {}: {e}", root.display());
        });
        let root = std::fs::canonicalize(&root).unwrap_or_else(|e| {
            panic!("sova-fs: canonicalize root {}: {e}", root.display());
        });

        app.state(Fs {
            inner: Arc::new(FsInner {
                root,
                max_walk_depth,
                max_walk_entries,
                events,
            }),
        });
    }
}
