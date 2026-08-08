//! Integration smoke against a live MinIO / S3 endpoint.
//!
//! ```bash
//! export RUVO_STORAGE_ENDPOINT=http://127.0.0.1:9000
//! export RUVO_STORAGE_BUCKET=ruvo
//! export AWS_ACCESS_KEY_ID=minioadmin
//! export AWS_SECRET_ACCESS_KEY=minioadmin
//! cargo test -p ruvo-storage --features s3 --test minio_smoke -- --ignored --nocapture
//! ```

#![cfg(feature = "s3")]

use bytes::Bytes;
use ruvo_storage::{s3_from_env, BlobStore, PutOpts};
use std::time::Duration;

#[tokio::test]
#[ignore = "requires MinIO/S3: set RUVO_STORAGE_ENDPOINT (+ bucket + keys)"]
async fn minio_put_get_delete() {
    if std::env::var("RUVO_STORAGE_ENDPOINT").is_err() {
        eprintln!("skip: RUVO_STORAGE_ENDPOINT unset");
        return;
    }

    let store = s3_from_env().expect("s3_from_env");
    let key = format!(
        "ruvo-smoke/{}/ping.txt",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    store
        .put(
            &key,
            Bytes::from_static(b"ruvo-minio-ok"),
            PutOpts {
                content_type: Some("text/plain".into()),
                ..Default::default()
            },
        )
        .await
        .expect("put");

    assert!(store.exists(&key).await.expect("exists"));
    assert_eq!(
        store.get(&key).await.expect("get").expect("bytes").as_ref(),
        b"ruvo-minio-ok"
    );

    let prefix = key.rsplit_once('/').map(|(p, _)| p).unwrap_or("ruvo-smoke");
    let listed = store.list(prefix).await.expect("list");
    assert!(listed.iter().any(|k| k == &key), "listed={listed:?}");

    let url = store
        .temporary_url(&key, Duration::from_secs(60))
        .await
        .expect("temporary_url");
    assert!(url.starts_with("http"), "{url}");

    store.delete(&key).await.expect("delete");
    assert!(!store.exists(&key).await.expect("exists after delete"));
}
