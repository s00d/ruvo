//! Storage plugin: from_env / from_config / AppStorage (local + memory only).

use bytes::Bytes;
use sova_core::{App, Request, Upload};
use sova_storage::{AppStorage, PutOpts, Storage, StorageExt};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn clear_storage_env() {
    for k in [
        "SOVA_STORAGE",
        "SOVA_STORAGE_PATH",
        "SOVA_STORAGE_PUBLIC_URL",
        "SOVA_STORAGE_BUCKET",
        "SOVA_STORAGE_REGION",
        "SOVA_STORAGE_ENDPOINT",
        "SOVA_STORAGE_ROOT",
        "SOVA_STORAGE_FORCE_PATH_STYLE",
    ] {
        std::env::remove_var(k);
    }
}

fn sample_upload(name: &str) -> Upload {
    Upload {
        field: "file".into(),
        filename: Some(name.into()),
        content_type: Some("image/png".into()),
        data: Bytes::from_static(b"png-bytes"),
    }
}

#[tokio::test]
async fn memory_store_put_get_list_delete_url() {
    let mut app = App::new();
    app.install(Storage::memory().public_url("https://cdn.example/files"));
    let storage = app.try_state::<AppStorage>().unwrap();

    storage
        .put("a/b.txt", Bytes::from_static(b"hi"), PutOpts::default())
        .await
        .unwrap();
    assert!(storage.exists("a/b.txt").await.unwrap());
    assert_eq!(
        storage.get("a/b.txt").await.unwrap().as_deref(),
        Some(b"hi".as_slice())
    );
    let listed = storage.list("a/").await.unwrap();
    assert!(listed.iter().any(|k| k == "a/b.txt"));
    assert_eq!(
        storage.url("a/b.txt").as_deref(),
        Some("https://cdn.example/files/a/b.txt")
    );

    assert!(storage
        .temporary_url("a/b.txt", Duration::from_secs(60))
        .await
        .is_err());
    assert!(storage
        .temporary_upload_url("a/b.txt", Duration::from_secs(60))
        .await
        .is_err());

    storage.delete("a/b.txt").await.unwrap();
    assert!(!storage.exists("a/b.txt").await.unwrap());
}

#[tokio::test]
async fn store_and_store_as_with_upload() {
    let mut app = App::new();
    app.install(Storage::memory().public_url("/assets"));
    let storage = app.try_state::<AppStorage>().unwrap();

    let stored = storage
        .store(&sample_upload("avatar.png"), "avatars")
        .await
        .unwrap();
    assert!(stored.key.starts_with("avatars/"));
    assert!(stored.key.ends_with(".png"));
    assert!(stored.url.as_ref().unwrap().starts_with("/assets/avatars/"));
    assert_eq!(
        storage.get(&stored.key).await.unwrap().as_deref(),
        Some(b"png-bytes".as_slice())
    );

    let exact = storage
        .store_as(&sample_upload("x.bin"), "exact/key.bin")
        .await
        .unwrap();
    assert_eq!(exact.key, "exact/key.bin");
    assert_eq!(exact.url.as_deref(), Some("/assets/exact/key.bin"));

    let root = storage.store(&sample_upload("n.ext"), "").await.unwrap();
    assert!(!root.key.contains('/'));
    assert!(root.key.ends_with(".ext"));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // env mutex serializes SOVA_STORAGE_* across tests
async fn from_env_memory_and_local() {
    let _guard = env_lock();
    clear_storage_env();
    std::env::set_var("SOVA_STORAGE", "memory");
    std::env::set_var("SOVA_STORAGE_PUBLIC_URL", "https://x.test");
    let mem = Storage::from_env().unwrap();
    let mut app = App::new();
    app.install(mem);
    let s = app.try_state::<AppStorage>().unwrap();
    assert_eq!(s.url("k").as_deref(), Some("https://x.test/k"));

    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("SOVA_STORAGE", "local");
    std::env::set_var("SOVA_STORAGE_PATH", dir.path().to_str().unwrap());
    std::env::remove_var("SOVA_STORAGE_PUBLIC_URL");
    let local = Storage::from_env().unwrap();
    let mut app2 = App::new();
    app2.install(local);
    let s2 = app2.try_state::<AppStorage>().unwrap();
    s2.put("f.txt", Bytes::from_static(b"ok"), PutOpts::default())
        .await
        .unwrap();
    assert_eq!(
        s2.get("f.txt").await.unwrap().as_deref(),
        Some(b"ok".as_slice())
    );

    std::env::set_var("SOVA_STORAGE", "nope");
    assert!(Storage::from_env().is_err());
    clear_storage_env();
}

#[tokio::test]
async fn from_config_driver_memory_and_toml_public_url() {
    let _guard = env_lock();
    clear_storage_env();
    let mut app = App::new();
    app.configure_from_str(
        r#"
[storage]
driver = "memory"
public_url = "https://from-toml"
"#,
    )
    .unwrap();
    let storage = Storage::from_config(&app).unwrap();
    app.install(storage);
    let s = app.try_state::<AppStorage>().unwrap();
    assert_eq!(s.url("a").as_deref(), Some("https://from-toml/a"));
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // env mutex serializes SOVA_STORAGE_* across tests
async fn from_config_local_path_and_unknown_driver() {
    let _guard = env_lock();
    clear_storage_env();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_str().unwrap();
    let mut app = App::new();
    app.configure_from_str(&format!(
        r#"
[storage]
driver = "local"
path = "{path}"
public_url = "https://cdn"
"#
    ))
    .unwrap();
    let storage = Storage::from_config(&app).unwrap();
    app.install(storage);
    let s = app.try_state::<AppStorage>().unwrap();
    s.put("z.txt", Bytes::from_static(b"z"), PutOpts::default())
        .await
        .unwrap();
    assert!(s.exists("z.txt").await.unwrap());

    clear_storage_env();
    let mut bad = App::new();
    bad.configure_from_str(
        r#"
[storage]
driver = "ftp"
"#,
    )
    .unwrap();
    assert!(Storage::from_config(&bad).is_err());
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // env mutex serializes SOVA_STORAGE_* across tests
async fn install_health_check_and_req_storage_ext() {
    let _guard = env_lock();
    clear_storage_env();
    let mut app = App::new();
    app.install(Storage::memory());
    app.with_probes();
    app.get("/ping", |req: Request| async move {
        let s = req.storage();
        s.put("p.txt", Bytes::from_static(b"1"), PutOpts::default())
            .await
            .unwrap();
        sova_core::Response::text("ok")
    });

    let c = sova_core::TestClient::tracked(app).await.unwrap();
    assert_eq!(c.get("/ping").await.status_code().as_u16(), 200);
    let health = c.get("/ready").await;
    assert_eq!(health.status_code().as_u16(), 200);

    let mut app2 = App::new();
    app2.configure_from_str(
        r#"
[storage]
public_url = "https://toml-only"
"#,
    )
    .unwrap();
    app2.install(Storage::memory().public_url("https://explicit"));
    let s = app2.try_state::<AppStorage>().unwrap();
    assert_eq!(s.url("x").as_deref(), Some("https://explicit/x"));
}

#[tokio::test]
async fn from_config_s3_without_feature_errors() {
    let _guard = env_lock();
    clear_storage_env();
    let mut app = App::new();
    app.configure_from_str(
        r#"
[storage]
driver = "s3"
"#,
    )
    .unwrap();
    assert!(
        Storage::from_config(&app).is_err(),
        "s3 without feature must error"
    );
}
