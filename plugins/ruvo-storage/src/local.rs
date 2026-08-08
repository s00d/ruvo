//! Local filesystem [`BlobStore`].

use crate::{normalize_key, normalize_prefix, BlobStore, BoxFuture, PutOpts, StorageError};
use bytes::Bytes;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct LocalStore {
    root: PathBuf,
}

impl LocalStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_for(&self, key: &str) -> Result<PathBuf, StorageError> {
        let key = normalize_key(key)?;
        Ok(self.root.join(key))
    }
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), StorageError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| StorageError::Msg(format!("list strip_prefix: {e}")))?;
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

impl BlobStore for LocalStore {
    fn put<'a>(
        &'a self,
        key: &'a str,
        data: Bytes,
        _opts: PutOpts,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let path = self.path_for(key)?;
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }
            tokio::fs::write(&path, &data).await?;
            Ok(())
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<Bytes>, StorageError>> {
        Box::pin(async move {
            let path = self.path_for(key)?;
            match tokio::fs::read(&path).await {
                Ok(bytes) => Ok(Some(Bytes::from(bytes))),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            }
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let path = self.path_for(key)?;
            match tokio::fs::remove_file(&path).await {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            }
        })
    }

    fn exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            let path = self.path_for(key)?;
            Ok(Path::new(&path).exists())
        })
    }

    fn list<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, Result<Vec<String>, StorageError>> {
        Box::pin(async move {
            let prefix = normalize_prefix(prefix)?;
            let start = if prefix.is_empty() {
                self.root.clone()
            } else {
                self.root.join(&prefix)
            };
            let mut keys = Vec::new();
            if start.is_file() {
                keys.push(prefix);
                return Ok(keys);
            }
            collect_files(&self.root, &start, &mut keys)?;
            // When prefix is a dir without trailing content yet, also accept keys under prefix/
            if !prefix.is_empty() && start.is_dir() {
                // already collected under start
            } else if !prefix.is_empty() && !start.exists() {
                // prefix may be "a" matching files "a/b" — walk root and filter
                keys.clear();
                collect_files(&self.root, &self.root, &mut keys)?;
                keys.retain(|k| k == &prefix || k.starts_with(&format!("{prefix}/")));
            }
            keys.sort();
            Ok(keys)
        })
    }
}
