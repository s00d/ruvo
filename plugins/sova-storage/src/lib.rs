//! Object storage for Sova — local disk, memory, S3/R2/MinIO, GCS, Azure.
//!
//! Upload pipeline:
//! ```ignore
//! file.validate(&UploadRules::new().max_bytes(2_000_000).extensions(["png", "jpg"]))?;
//! let stored = req.storage().store(&file, "avatars").await?;
//! // stored.key / stored.url
//! ```

mod error;
mod local;
mod memory;
mod plugin;

#[cfg(any(feature = "s3", feature = "gcs", feature = "azure"))]
mod opendal_store;

#[cfg(feature = "azure")]
pub use opendal_store::azure_from_env;
#[cfg(feature = "gcs")]
pub use opendal_store::gcs_from_env;
#[cfg(any(feature = "s3", feature = "gcs", feature = "azure"))]
pub use opendal_store::OpendalStore;
#[cfg(feature = "s3")]
pub use opendal_store::{s3_from_env, S3EnvConfig};

pub use error::StorageError;
pub use local::LocalStore;
pub use memory::MemoryStore;
pub use plugin::{AppStorage, Storage, StorageExt, StoredFile};

use bytes::Bytes;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Default)]
pub struct PutOpts {
    pub content_type: Option<String>,
    pub content_disposition: Option<String>,
    pub cache_control: Option<String>,
    /// User metadata key/value pairs (cloud backends).
    pub metadata: Vec<(String, String)>,
}

/// Byte-oriented object store. Custom backends: `impl BlobStore` + [`Storage::new`].
pub trait BlobStore: Send + Sync + 'static {
    fn put<'a>(
        &'a self,
        key: &'a str,
        data: Bytes,
        opts: PutOpts,
    ) -> BoxFuture<'a, Result<(), StorageError>>;

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<Bytes>, StorageError>>;

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), StorageError>>;

    fn exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>>;

    /// List object keys under `prefix` (empty = store root). Returns file keys only.
    fn list<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, Result<Vec<String>, StorageError>>;

    /// Laravel-style temporary download URL (presigned GET). Default: unsupported.
    fn temporary_url<'a>(
        &'a self,
        _key: &'a str,
        _expires: Duration,
    ) -> BoxFuture<'a, Result<String, StorageError>> {
        Box::pin(async {
            Err(StorageError::Msg(
                "temporary_url requires s3/gcs/azure backend".into(),
            ))
        })
    }

    /// Laravel-style temporary upload URL (presigned PUT). Default: unsupported.
    fn temporary_upload_url<'a>(
        &'a self,
        _key: &'a str,
        _expires: Duration,
    ) -> BoxFuture<'a, Result<String, StorageError>> {
        Box::pin(async {
            Err(StorageError::Msg(
                "temporary_upload_url requires s3/gcs/azure backend".into(),
            ))
        })
    }
}

/// Normalize object keys: trim leading `/`, reject `..`.
pub fn normalize_key(key: &str) -> Result<String, StorageError> {
    let key = key.trim().trim_start_matches('/');
    if key.is_empty() {
        return Err(StorageError::Msg("empty storage key".into()));
    }
    if key.split('/').any(|p| p == ".." || p == ".") {
        return Err(StorageError::Msg("unsafe storage key".into()));
    }
    Ok(key.to_string())
}

/// Normalize a list prefix: empty allowed (store root); reject `..`.
pub fn normalize_prefix(prefix: &str) -> Result<String, StorageError> {
    let prefix = prefix.trim().trim_start_matches('/');
    if prefix.is_empty() {
        return Ok(String::new());
    }
    if prefix.split('/').any(|p| p == ".." || p == ".") {
        return Err(StorageError::Msg("unsafe storage prefix".into()));
    }
    Ok(prefix.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn memory_roundtrip() {
        let store = MemoryStore::new();
        store
            .put("a/b.txt", Bytes::from_static(b"hi"), PutOpts::default())
            .await
            .unwrap();
        assert!(store.exists("a/b.txt").await.unwrap());
        assert_eq!(store.get("a/b.txt").await.unwrap().unwrap().as_ref(), b"hi");
        store.delete("a/b.txt").await.unwrap();
        assert!(!store.exists("a/b.txt").await.unwrap());
        assert!(store.get("a/b.txt").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn memory_list_and_temporary_url_unsupported() {
        let store = MemoryStore::new();
        store
            .put("a/1.txt", Bytes::from_static(b"1"), PutOpts::default())
            .await
            .unwrap();
        store
            .put("a/2.txt", Bytes::from_static(b"2"), PutOpts::default())
            .await
            .unwrap();
        store
            .put("b/3.txt", Bytes::from_static(b"3"), PutOpts::default())
            .await
            .unwrap();
        let mut keys = store.list("a").await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["a/1.txt", "a/2.txt"]);

        let err = store
            .temporary_url("a/1.txt", Duration::from_secs(60))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("temporary_url"));
    }

    #[tokio::test]
    async fn local_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalStore::new(dir.path());
        store
            .put(
                "x/y.bin",
                Bytes::from_static(b"data"),
                PutOpts {
                    content_type: Some("application/octet-stream".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(dir.path().join("x/y.bin").is_file());
        assert_eq!(
            store.get("x/y.bin").await.unwrap().unwrap().as_ref(),
            b"data"
        );
        let listed = store.list("x").await.unwrap();
        assert_eq!(listed, vec!["x/y.bin"]);
        store.delete("x/y.bin").await.unwrap();
        assert!(!store.exists("x/y.bin").await.unwrap());
    }

    #[tokio::test]
    async fn rejects_unsafe_key() {
        let store = MemoryStore::new();
        let err = store
            .put("../x", Bytes::from_static(b"no"), PutOpts::default())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("unsafe"));
    }

    #[tokio::test]
    async fn app_storage_url() {
        let app = AppStorage::new(Arc::new(MemoryStore::new())).with_public_url("/assets/uploads");
        assert_eq!(
            app.url("avatars/u1.png").as_deref(),
            Some("/assets/uploads/avatars/u1.png")
        );
    }

    #[tokio::test]
    async fn store_and_store_as() {
        use sova_core::Upload;

        let app = AppStorage::new(Arc::new(MemoryStore::new())).with_public_url("/assets");
        let upload = Upload {
            field: "file".into(),
            filename: Some("photo.PNG".into()),
            content_type: Some("image/png".into()),
            data: Bytes::from_static(b"png-bytes"),
        };

        let stored = app.store(&upload, "avatars").await.unwrap();
        assert!(stored.key.starts_with("avatars/"));
        assert!(stored.key.ends_with(".png"));
        assert_eq!(
            stored.url.as_deref(),
            Some(format!("/assets/{}", stored.key).as_str())
        );
        assert_eq!(
            app.get(&stored.key).await.unwrap().unwrap().as_ref(),
            b"png-bytes"
        );

        let fixed = app.store_as(&upload, "avatars/u42.png").await.unwrap();
        assert_eq!(fixed.key, "avatars/u42.png");
        assert_eq!(fixed.url.as_deref(), Some("/assets/avatars/u42.png"));
    }
}
