//! OpenDAL-backed [`BlobStore`] (S3 / GCS / Azure).
//!
//! Env builders are split so unit tests can feed a fake env map without mutating
//! process environment (and without hitting the network).

use crate::{normalize_key, normalize_prefix, BlobStore, BoxFuture, PutOpts, StorageError};
use bytes::Bytes;
use opendal::Operator;
use std::time::Duration;

#[derive(Clone)]
pub struct OpendalStore {
    op: Operator,
}

impl OpendalStore {
    pub fn new(op: Operator) -> Self {
        Self { op }
    }
}

impl BlobStore for OpendalStore {
    fn put<'a>(
        &'a self,
        key: &'a str,
        data: Bytes,
        opts: PutOpts,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            let rich = opts.content_type.is_some()
                || opts.content_disposition.is_some()
                || opts.cache_control.is_some()
                || !opts.metadata.is_empty();
            if !rich {
                self.op.write(&key, data.to_vec()).await?;
                return Ok(());
            }
            let mut w = self.op.write_with(&key, data.to_vec());
            if let Some(ct) = opts.content_type.as_deref() {
                w = w.content_type(ct);
            }
            if let Some(cd) = opts.content_disposition.as_deref() {
                w = w.content_disposition(cd);
            }
            if let Some(cc) = opts.cache_control.as_deref() {
                w = w.cache_control(cc);
            }
            if !opts.metadata.is_empty() {
                w = w.user_metadata(opts.metadata);
            }
            w.await?;
            Ok(())
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<Bytes>, StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            match self.op.read(&key).await {
                Ok(buf) => Ok(Some(Bytes::copy_from_slice(buf.to_bytes().as_ref()))),
                Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            }
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            match self.op.delete(&key).await {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            }
        })
    }

    fn exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            match self.op.stat(&key).await {
                Ok(_) => Ok(true),
                Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(e.into()),
            }
        })
    }

    fn list<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, Result<Vec<String>, StorageError>> {
        Box::pin(async move {
            let prefix = normalize_prefix(prefix)?;
            let path = if prefix.is_empty() {
                "/".to_string()
            } else if prefix.ends_with('/') {
                prefix.clone()
            } else {
                format!("{prefix}/")
            };
            let entries = self.op.list(&path).await?;
            let mut keys = Vec::new();
            for entry in entries {
                if entry.metadata().is_file() {
                    let p = entry.path().trim_start_matches('/').to_string();
                    if !p.is_empty() {
                        keys.push(p);
                    }
                }
            }
            // Exact key match when prefix is a file path
            if !prefix.is_empty() && !prefix.ends_with('/') && self.exists(&prefix).await? {
                if !keys.iter().any(|k| k == &prefix) {
                    keys.push(prefix);
                }
            }
            keys.sort();
            keys.dedup();
            Ok(keys)
        })
    }

    fn temporary_url<'a>(
        &'a self,
        key: &'a str,
        expires: Duration,
    ) -> BoxFuture<'a, Result<String, StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            let req = self.op.presign_read(&key, expires).await?;
            Ok(req.uri().to_string())
        })
    }

    fn temporary_upload_url<'a>(
        &'a self,
        key: &'a str,
        expires: Duration,
    ) -> BoxFuture<'a, Result<String, StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            let req = self.op.presign_write(&key, expires).await?;
            Ok(req.uri().to_string())
        })
    }
}

fn env_get(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

fn env_truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn env_falsy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

/// Parsed S3/R2/MinIO settings from env (testable without OpenDAL I/O).
#[cfg(feature = "s3")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3EnvConfig {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub root: Option<String>,
    /// Path-style addressing (MinIO/custom endpoints). When `false`, virtual-host style.
    pub force_path_style: bool,
}

#[cfg(feature = "s3")]
impl S3EnvConfig {
    /// Read process env (`RUVO_STORAGE_*` / `AWS_*`).
    pub fn from_env() -> Result<Self, StorageError> {
        Self::from_vars(env_get)
    }

    /// Build from a lookup fn (unit tests pass a `HashMap` closure).
    pub fn from_vars<F>(mut get: F) -> Result<Self, StorageError>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let bucket = get("RUVO_STORAGE_BUCKET")
            .or_else(|| get("AWS_BUCKET"))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                StorageError::Msg(
                    "RUVO_STORAGE_BUCKET (or AWS_BUCKET) is required for s3".into(),
                )
            })?;

        let endpoint = get("RUVO_STORAGE_ENDPOINT").filter(|s| !s.is_empty());

        let region = get("RUVO_STORAGE_REGION")
            .or_else(|| get("AWS_REGION"))
            .filter(|s| !s.is_empty())
            .or_else(|| endpoint.as_ref().map(|_| "auto".to_string()))
            .ok_or_else(|| {
                StorageError::Msg(
                    "RUVO_STORAGE_REGION or AWS_REGION is required for s3 \
                     (or set RUVO_STORAGE_ENDPOINT for MinIO/R2 to default region=auto)"
                        .into(),
                )
            })?;

        // Default: path-style when a custom endpoint is set (MinIO); virtual-host for plain AWS.
        let force_path_style = match get("RUVO_STORAGE_FORCE_PATH_STYLE") {
            Some(raw) if env_truthy(&raw) => true,
            Some(raw) if env_falsy(&raw) => false,
            Some(raw) => {
                return Err(StorageError::Msg(format!(
                    "invalid RUVO_STORAGE_FORCE_PATH_STYLE={raw} (use 1/0, true/false)"
                )));
            }
            None => endpoint.is_some(),
        };

        Ok(Self {
            bucket,
            region,
            endpoint,
            access_key_id: get("AWS_ACCESS_KEY_ID").filter(|s| !s.is_empty()),
            secret_access_key: get("AWS_SECRET_ACCESS_KEY").filter(|s| !s.is_empty()),
            session_token: get("AWS_SESSION_TOKEN")
                .or_else(|| get("RUVO_STORAGE_SESSION_TOKEN"))
                .filter(|s| !s.is_empty()),
            root: get("RUVO_STORAGE_ROOT").filter(|s| !s.is_empty()),
            force_path_style,
        })
    }

    /// Build an OpenDAL S3 operator from this config.
    pub fn build(self) -> Result<OpendalStore, StorageError> {
        use opendal::services::S3;

        let mut builder = S3::default()
            .bucket(&self.bucket)
            .region(&self.region);

        if let Some(endpoint) = self.endpoint.as_deref() {
            builder = builder.endpoint(endpoint);
        }
        if let Some(root) = self.root.as_deref() {
            builder = builder.root(root);
        }
        if let Some(key) = self.access_key_id.as_deref() {
            builder = builder.access_key_id(key);
        }
        if let Some(secret) = self.secret_access_key.as_deref() {
            builder = builder.secret_access_key(secret);
        }
        if let Some(token) = self.session_token.as_deref() {
            builder = builder.session_token(token);
        }
        // OpenDAL defaults to path-style; enable virtual-host when path-style is off.
        if !self.force_path_style {
            builder = builder.enable_virtual_host_style();
        }

        let op = Operator::new(builder)?;
        Ok(OpendalStore::new(op))
    }
}

/// S3 / R2 / MinIO from env. See [`S3EnvConfig`].
#[cfg(feature = "s3")]
pub fn s3_from_env() -> Result<OpendalStore, StorageError> {
    S3EnvConfig::from_env()?.build()
}

#[cfg(feature = "gcs")]
pub fn gcs_from_env() -> Result<OpendalStore, StorageError> {
    use opendal::services::Gcs;

    let bucket = env_get("RUVO_STORAGE_BUCKET").ok_or_else(|| {
        StorageError::Msg("RUVO_STORAGE_BUCKET is required for gcs".into())
    })?;

    let mut builder = Gcs::default().bucket(&bucket);
    if let Some(cred) = env_get("GOOGLE_APPLICATION_CREDENTIALS") {
        builder = builder.credential_path(&cred);
    }
    if let Some(root) = env_get("RUVO_STORAGE_ROOT") {
        builder = builder.root(&root);
    }
    if let Some(endpoint) = env_get("RUVO_STORAGE_ENDPOINT") {
        builder = builder.endpoint(&endpoint);
    }

    let op = Operator::new(builder)?;
    Ok(OpendalStore::new(op))
}

#[cfg(feature = "azure")]
pub fn azure_from_env() -> Result<OpendalStore, StorageError> {
    use opendal::services::Azblob;

    let container = env_get("RUVO_STORAGE_CONTAINER")
        .or_else(|| env_get("RUVO_STORAGE_BUCKET"))
        .ok_or_else(|| {
            StorageError::Msg(
                "RUVO_STORAGE_CONTAINER (or RUVO_STORAGE_BUCKET) required for azure".into(),
            )
        })?;

    let mut builder = Azblob::default().container(&container);
    if let Some(account) = env_get("AZURE_STORAGE_ACCOUNT_NAME") {
        builder = builder.account_name(&account);
    }
    if let Some(key) = env_get("AZURE_STORAGE_ACCOUNT_KEY") {
        builder = builder.account_key(&key);
    }
    // Azurite / custom: AZURE_STORAGE_ENDPOINT preferred, else RUVO_STORAGE_ENDPOINT.
    if let Some(endpoint) =
        env_get("AZURE_STORAGE_ENDPOINT").or_else(|| env_get("RUVO_STORAGE_ENDPOINT"))
    {
        builder = builder.endpoint(&endpoint);
    }
    if let Some(root) = env_get("RUVO_STORAGE_ROOT") {
        builder = builder.root(&root);
    }

    let op = Operator::new(builder)?;
    Ok(OpendalStore::new(op))
}

#[cfg(all(test, feature = "s3"))]
mod s3_env_tests {
    use super::*;
    use std::collections::HashMap;

    fn map_get<'a>(
        map: &'a HashMap<&'static str, &'static str>,
    ) -> impl FnMut(&str) -> Option<String> + 'a {
        move |k| map.get(k).map(|s| (*s).to_string())
    }

    #[test]
    fn minio_endpoint_defaults_region_auto_and_path_style() {
        let mut env = HashMap::new();
        env.insert("RUVO_STORAGE_BUCKET", "demo");
        env.insert("RUVO_STORAGE_ENDPOINT", "http://127.0.0.1:9000");
        env.insert("AWS_ACCESS_KEY_ID", "minio");
        env.insert("AWS_SECRET_ACCESS_KEY", "minio123");

        let cfg = S3EnvConfig::from_vars(map_get(&env)).unwrap();
        assert_eq!(cfg.region, "auto");
        assert!(cfg.force_path_style);
        assert_eq!(cfg.endpoint.as_deref(), Some("http://127.0.0.1:9000"));
    }

    #[test]
    fn aws_requires_region_without_endpoint() {
        let mut env = HashMap::new();
        env.insert("RUVO_STORAGE_BUCKET", "demo");
        env.insert("AWS_ACCESS_KEY_ID", "ak");
        env.insert("AWS_SECRET_ACCESS_KEY", "sk");

        let err = S3EnvConfig::from_vars(map_get(&env)).unwrap_err();
        assert!(err.to_string().contains("REGION"));
    }

    #[test]
    fn aws_virtual_host_when_region_set() {
        let mut env = HashMap::new();
        env.insert("RUVO_STORAGE_BUCKET", "demo");
        env.insert("AWS_REGION", "eu-west-1");

        let cfg = S3EnvConfig::from_vars(map_get(&env)).unwrap();
        assert_eq!(cfg.region, "eu-west-1");
        assert!(!cfg.force_path_style);
    }

    #[test]
    fn force_path_style_override_and_session_token() {
        let mut env = HashMap::new();
        env.insert("RUVO_STORAGE_BUCKET", "demo");
        env.insert("AWS_REGION", "us-east-1");
        env.insert("RUVO_STORAGE_FORCE_PATH_STYLE", "1");
        env.insert("AWS_SESSION_TOKEN", "tok");
        env.insert("RUVO_STORAGE_ROOT", "uploads");

        let cfg = S3EnvConfig::from_vars(map_get(&env)).unwrap();
        assert!(cfg.force_path_style);
        assert_eq!(cfg.session_token.as_deref(), Some("tok"));
        assert_eq!(cfg.root.as_deref(), Some("uploads"));
    }

    #[test]
    fn empty_bucket_rejected() {
        let mut env = HashMap::new();
        env.insert("RUVO_STORAGE_BUCKET", "");
        env.insert("AWS_REGION", "us-east-1");
        let err = S3EnvConfig::from_vars(map_get(&env)).unwrap_err();
        assert!(err.to_string().contains("BUCKET"));
    }
}
