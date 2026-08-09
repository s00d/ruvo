//! Db plugin with sqlite (memory / tempfile).
#![cfg(feature = "sqlite")]

use sova_core::{App, TestClient};
use sova_db::{Db, DbPool};
use tempfile::NamedTempFile;

#[tokio::test]
async fn sqlite_memory_install_startup_and_pool() {
    let mut app = App::new();
    app.install(Db::from_env().url("sqlite::memory:"));
    app.with_probes();

    let state = app.run_startup().await.expect("run_startup");
    let pool = state.get::<DbPool>().expect("DbPool in state");
    let conn = pool.get().await.expect("connected");
    conn.ping().await.expect("ping");

    let c = TestClient::tracked(app).await.unwrap();
    let ready = c.get("/ready").await;
    assert_eq!(ready.status_code().as_u16(), 200);
    let body = String::from_utf8_lossy(ready.body_bytes().unwrap_or(b""));
    assert!(body.contains(r#""status":"ok"#), "body={body}");
    assert!(body.contains("db"), "body={body}");
}

#[tokio::test]
async fn sqlite_tempfile_url() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_string_lossy();
    let url = format!("sqlite://{path}?mode=rwc");

    let mut app = App::new();
    app.install(Db::from_env().url(url));
    let state = app.run_startup().await.expect("run_startup tempfile");
    let pool = state.get::<DbPool>().expect("DbPool");
    pool.get().await.expect("connected").ping().await.unwrap();
}

#[tokio::test]
async fn url_from_toml_when_builder_empty() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_string_lossy();
    let url = format!("sqlite://{path}?mode=rwc");
    let prev = std::env::var("DATABASE_URL").ok();
    std::env::remove_var("DATABASE_URL");

    let mut app = App::new();
    app.configure_from_str(&format!(
        r#"
[db]
url = "{url}"
"#
    ))
    .unwrap();
    app.install(Db::from_env().url(""));
    let state = app.run_startup().await.expect("toml url startup");
    let pool = state.get::<DbPool>().expect("DbPool");
    pool.get().await.expect("connected").ping().await.unwrap();

    match prev {
        Some(v) => std::env::set_var("DATABASE_URL", v),
        None => std::env::remove_var("DATABASE_URL"),
    }
}
