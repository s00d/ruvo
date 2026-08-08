//! [`Storage`] plugin + [`AppStorage`] handle.

use crate::{normalize_key, BlobStore, LocalStore, MemoryStore, PutOpts, StorageError};
use bytes::Bytes;
use rand::RngCore;
use sova_core::{App, Plugin, Request, Upload};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Result of [`AppStorage::store`] / [`AppStorage::store_as`].
#[derive(Debug, Clone)]
pub struct StoredFile {
    pub key: String,
    /// Public URL when `public_url` was configured on the plugin.
    pub url: Option<String>,
}

/// App-state handle: backend + optional public URL prefix.
#[derive(Clone)]
pub struct AppStorage {
    pub inner: Arc<dyn BlobStore>,
    public_base: Option<String>,
}

impl AppStorage {
    pub fn new(store: Arc<dyn BlobStore>) -> Self {
        Self {
            inner: store,
            public_base: None,
        }
    }

    pub fn with_public_url(mut self, base: impl Into<String>) -> Self {
        let base = base.into().trim_end_matches('/').to_string();
        self.public_base = if base.is_empty() { None } else { Some(base) };
        self
    }

    /// Public URL for `key` when `public_url` was set (e.g. `/assets/uploads/avatars/u1.png`).
    pub fn url(&self, key: &str) -> Option<String> {
        let key = normalize_key(key).ok()?;
        let base = self.public_base.as_ref()?;
        Some(format!("{base}/{key}"))
    }

    pub async fn put(&self, key: &str, data: Bytes, opts: PutOpts) -> Result<(), StorageError> {
        self.inner.put(key, data, opts).await
    }

    pub async fn get(&self, key: &str) -> Result<Option<Bytes>, StorageError> {
        self.inner.get(key).await
    }

    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.inner.delete(key).await
    }

    pub async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        self.inner.exists(key).await
    }

    pub async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        self.inner.list(prefix).await
    }

    /// Presigned download URL (S3/GCS/Azure). Err on local/memory.
    pub async fn temporary_url(
        &self,
        key: &str,
        expires: Duration,
    ) -> Result<String, StorageError> {
        self.inner.temporary_url(key, expires).await
    }

    /// Presigned upload URL (S3/GCS/Azure). Err on local/memory.
    pub async fn temporary_upload_url(
        &self,
        key: &str,
        expires: Duration,
    ) -> Result<String, StorageError> {
        self.inner.temporary_upload_url(key, expires).await
    }

    /// Store an upload's bytes under `key` (uses upload content-type when present).
    pub async fn put_upload(&self, key: &str, upload: &Upload) -> Result<(), StorageError> {
        let opts = PutOpts {
            content_type: upload.content_type.clone(),
            ..Default::default()
        };
        self.put(key, upload.data.clone(), opts).await
    }

    /// Store under `dir` + random name (extension from upload, fallback `bin`).
    ///
    /// Validate with [`sova_core::UploadRules`] before calling.
    pub async fn store(&self, upload: &Upload, dir: &str) -> Result<StoredFile, StorageError> {
        let dir = dir.trim().trim_matches('/');
        let name = random_object_name(upload);
        let key = if dir.is_empty() {
            name
        } else {
            format!("{dir}/{name}")
        };
        self.store_as(upload, &key).await
    }

    /// Store under an exact key; returns key + optional public URL.
    pub async fn store_as(&self, upload: &Upload, key: &str) -> Result<StoredFile, StorageError> {
        let key = normalize_key(key)?;
        self.put_upload(&key, upload).await?;
        Ok(StoredFile {
            url: self.url(&key),
            key,
        })
    }
}

fn random_object_name(upload: &Upload) -> String {
    let mut buf = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut buf);
    let mut name = String::with_capacity(32 + 1 + 8);
    for b in buf {
        name.push_str(&format!("{b:02x}"));
    }
    let ext = upload.extension().unwrap_or_else(|| "bin".to_string());
    name.push('.');
    name.push_str(&ext);
    name
}

/// Convenient `req.storage()`.
pub trait StorageExt {
    fn storage(&self) -> AppStorage;
}

impl StorageExt for Request {
    fn storage(&self) -> AppStorage {
        self.try_state::<AppStorage>()
            .map(|a| (*a).clone())
            .expect("Storage plugin is not installed (missing req.storage())")
    }
}

/// Plugin that installs [`AppStorage`].
pub struct Storage {
    store: Arc<dyn BlobStore>,
    public_base: Option<String>,
    public_url_explicit: bool,
}

impl Storage {
    pub fn new(store: Arc<dyn BlobStore>) -> Self {
        Self {
            store,
            public_base: None,
            public_url_explicit: false,
        }
    }

    #[cfg(feature = "local")]
    pub fn local(root: impl Into<PathBuf>) -> Self {
        Self::new(Arc::new(LocalStore::new(root)))
    }

    #[cfg(feature = "memory")]
    pub fn memory() -> Self {
        Self::new(Arc::new(MemoryStore::new()))
    }

    pub fn public_url(mut self, base: impl Into<String>) -> Self {
        let base = base.into().trim_end_matches('/').to_string();
        self.public_base = if base.is_empty() { None } else { Some(base) };
        self.public_url_explicit = true;
        self
    }

    /// Build from `[storage]` on `app` (after `configure`) with env overrides for secrets.
    ///
    /// Keys: `driver`, `path`, `public_url`, plus env `SOVA_STORAGE*` / cloud credentials.
    pub fn from_config(app: &App) -> Result<Self, StorageError> {
        let section = app.config_doc().and_then(|d| d.section("storage"));
        let driver = std::env::var("SOVA_STORAGE")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                section
                    .as_ref()
                    .and_then(|s| s.get("driver").and_then(|v| v.as_str()).map(str::to_string))
            })
            .unwrap_or_else(|| "local".into());

        // Temporarily set env from toml for opendal helpers that only read env,
        // without clobbering already-set process env.
        let _guard = TomlEnvGuard::apply(section.as_ref());

        let mut storage = match driver.as_str() {
            "local" => {
                #[cfg(feature = "local")]
                {
                    let path = std::env::var("SOVA_STORAGE_PATH")
                        .ok()
                        .filter(|s| !s.is_empty())
                        .or_else(|| {
                            section.as_ref().and_then(|s| {
                                s.get("path").and_then(|v| v.as_str()).map(str::to_string)
                            })
                        })
                        .unwrap_or_else(|| "./storage".into());
                    Ok(Self::local(path))
                }
                #[cfg(not(feature = "local"))]
                {
                    Err(StorageError::Msg(
                        "storage driver=local requires feature `local`".into(),
                    ))
                }
            }
            "memory" => {
                #[cfg(feature = "memory")]
                {
                    Ok(Self::memory())
                }
                #[cfg(not(feature = "memory"))]
                {
                    Err(StorageError::Msg(
                        "storage driver=memory requires feature `memory`".into(),
                    ))
                }
            }
            "s3" => {
                #[cfg(feature = "s3")]
                {
                    Ok(Self::new(Arc::new(crate::opendal_store::s3_from_env()?)))
                }
                #[cfg(not(feature = "s3"))]
                {
                    Err(StorageError::Msg(
                        "storage driver=s3 requires feature `s3` (facade: storage-s3)".into(),
                    ))
                }
            }
            "gcs" => {
                #[cfg(feature = "gcs")]
                {
                    Ok(Self::new(Arc::new(crate::opendal_store::gcs_from_env()?)))
                }
                #[cfg(not(feature = "gcs"))]
                {
                    Err(StorageError::Msg(
                        "storage driver=gcs requires feature `gcs` (facade: storage-gcs)".into(),
                    ))
                }
            }
            "azure" => {
                #[cfg(feature = "azure")]
                {
                    Ok(Self::new(Arc::new(crate::opendal_store::azure_from_env()?)))
                }
                #[cfg(not(feature = "azure"))]
                {
                    Err(StorageError::Msg(
                        "storage driver=azure requires feature `azure` (facade: storage-azure)"
                            .into(),
                    ))
                }
            }
            other => Err(StorageError::Msg(format!(
                "unknown storage driver={other} (use local|memory|s3|gcs|azure)"
            ))),
        }?;

        let public = std::env::var("SOVA_STORAGE_PUBLIC_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                section.as_ref().and_then(|s| {
                    s.get("public_url")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
            });
        if let Some(base) = public {
            storage = storage.public_url(base);
        }
        drop(_guard);
        Ok(storage)
    }

    /// Build from `SOVA_STORAGE` (`local` | `memory` | `s3` | `gcs` | `azure`).
    pub fn from_env() -> Result<Self, StorageError> {
        let kind = std::env::var("SOVA_STORAGE").unwrap_or_else(|_| "local".into());
        let mut storage = match kind.as_str() {
            "local" => {
                #[cfg(feature = "local")]
                {
                    let path = std::env::var("SOVA_STORAGE_PATH")
                        .unwrap_or_else(|_| "./storage".into());
                    Ok(Self::local(path))
                }
                #[cfg(not(feature = "local"))]
                {
                    Err(StorageError::Msg(
                        "SOVA_STORAGE=local requires feature `local`".into(),
                    ))
                }
            }
            "memory" => {
                #[cfg(feature = "memory")]
                {
                    Ok(Self::memory())
                }
                #[cfg(not(feature = "memory"))]
                {
                    Err(StorageError::Msg(
                        "SOVA_STORAGE=memory requires feature `memory`".into(),
                    ))
                }
            }
            "s3" => {
                #[cfg(feature = "s3")]
                {
                    Ok(Self::new(Arc::new(crate::opendal_store::s3_from_env()?)))
                }
                #[cfg(not(feature = "s3"))]
                {
                    Err(StorageError::Msg(
                        "SOVA_STORAGE=s3 requires feature `s3` (facade: storage-s3)".into(),
                    ))
                }
            }
            "gcs" => {
                #[cfg(feature = "gcs")]
                {
                    Ok(Self::new(Arc::new(crate::opendal_store::gcs_from_env()?)))
                }
                #[cfg(not(feature = "gcs"))]
                {
                    Err(StorageError::Msg(
                        "SOVA_STORAGE=gcs requires feature `gcs` (facade: storage-gcs)".into(),
                    ))
                }
            }
            "azure" => {
                #[cfg(feature = "azure")]
                {
                    Ok(Self::new(Arc::new(crate::opendal_store::azure_from_env()?)))
                }
                #[cfg(not(feature = "azure"))]
                {
                    Err(StorageError::Msg(
                        "SOVA_STORAGE=azure requires feature `azure` (facade: storage-azure)"
                            .into(),
                    ))
                }
            }
            other => Err(StorageError::Msg(format!(
                "unknown SOVA_STORAGE={other} (use local|memory|s3|gcs|azure)"
            ))),
        }?;

        if let Ok(base) = std::env::var("SOVA_STORAGE_PUBLIC_URL") {
            storage = storage.public_url(base);
        }
        Ok(storage)
    }

    #[cfg(feature = "s3")]
    pub fn s3_from_env() -> Result<Self, StorageError> {
        Ok(Self::new(Arc::new(crate::opendal_store::s3_from_env()?)))
    }

    #[cfg(feature = "gcs")]
    pub fn gcs_from_env() -> Result<Self, StorageError> {
        Ok(Self::new(Arc::new(crate::opendal_store::gcs_from_env()?)))
    }

    #[cfg(feature = "azure")]
    pub fn azure_from_env() -> Result<Self, StorageError> {
        Ok(Self::new(Arc::new(crate::opendal_store::azure_from_env()?)))
    }
}

/// Set missing env vars from toml `[storage]` for cloud helpers; restore on drop.
struct TomlEnvGuard {
    restored: Vec<(String, Option<String>)>,
}

impl TomlEnvGuard {
    fn apply(section: Option<&toml::map::Map<String, toml::Value>>) -> Self {
        let mut restored = Vec::new();
        let Some(section) = section else {
            return Self { restored };
        };
        let pairs = [
            ("bucket", "SOVA_STORAGE_BUCKET"),
            ("region", "SOVA_STORAGE_REGION"),
            ("endpoint", "SOVA_STORAGE_ENDPOINT"),
            ("root", "SOVA_STORAGE_ROOT"),
            ("force_path_style", "SOVA_STORAGE_FORCE_PATH_STYLE"),
            ("path", "SOVA_STORAGE_PATH"),
            ("public_url", "SOVA_STORAGE_PUBLIC_URL"),
        ];
        for (toml_key, env_key) in pairs {
            if std::env::var(env_key).is_ok() {
                continue;
            }
            let val = match section.get(toml_key) {
                Some(toml::Value::String(s)) => s.clone(),
                Some(toml::Value::Boolean(b)) => b.to_string(),
                Some(toml::Value::Integer(i)) => i.to_string(),
                _ => continue,
            };
            let prev = std::env::var(env_key).ok();
            std::env::set_var(env_key, &val);
            restored.push((env_key.to_string(), prev));
        }
        Self { restored }
    }
}

impl Drop for TomlEnvGuard {
    fn drop(&mut self) {
        for (key, prev) in self.restored.drain(..) {
            match prev {
                Some(v) => std::env::set_var(&key, v),
                None => std::env::remove_var(&key),
            }
        }
    }
}

impl Plugin for Storage {
    fn id(&self) -> &'static str {
        "storage"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Storage")
            .description("Object storage (local / memory / S3 / GCS / Azure)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if !self.public_url_explicit {
            if let Some(doc) = app.config_doc() {
                if let Some(section) = doc.section("storage") {
                    if let Some(base) = section.get("public_url").and_then(|v| v.as_str()) {
                        let base = base.trim_end_matches('/').to_string();
                        self.public_base = if base.is_empty() { None } else { Some(base) };
                    }
                }
            }
        }
        let handle = AppStorage {
            inner: self.store,
            public_base: self.public_base,
        };
        let check = handle.clone();
        app.state(handle);
        app.register_check("storage", move |_state| {
            let check = check.clone();
            async move {
                // Lightweight ping: exists on a sentinel key (ok if missing).
                check
                    .exists("__sova_storage_health__")
                    .await
                    .map(|_| ())
                    .map_err(|e| sova_core::Error::Internal(e.to_string()))
            }
        });
    }
}
