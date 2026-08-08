//! Observability config: toml metrics_path vs builder override.

use ruvo_core::{App, Html, Request, TestClient};
use ruvo_observability::Observability;

#[tokio::test]
async fn metrics_path_from_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[observability]
metrics_path = "/custom-metrics"
"#,
    )
    .unwrap();
    app.install(Observability::new());
    app.get("/ping", |_r: Request| async { Html("pong") });

    let c = TestClient::tracked(app).unwrap();
    let _ = c.get("/ping").await;
    let metrics = c.get("/custom-metrics").await;
    assert_eq!(metrics.status_code().as_u16(), 200);
    let body = String::from_utf8(metrics.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("http_requests_total"), "body={body}");
}

#[tokio::test]
async fn builder_metrics_path_overrides_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[observability]
metrics_path = "/from-toml"
"#,
    )
    .unwrap();
    app.install(Observability::new().metrics_path("/from-builder"));
    app.get("/ping", |_r: Request| async { Html("pong") });

    let c = TestClient::tracked(app).unwrap();
    let _ = c.get("/ping").await;

    let builder = c.get("/from-builder").await;
    assert_eq!(builder.status_code().as_u16(), 200);

    let toml_path = c.get("/from-toml").await;
    assert_eq!(
        toml_path.status_code().as_u16(),
        404,
        "toml path must not win over builder"
    );
}

/// When feature `otel` is on, `[observability] otel = true` is applied; missing
/// endpoint only warns (install still succeeds).
#[cfg(feature = "otel")]
#[tokio::test]
async fn otel_bool_from_toml_does_not_panic() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[observability]
otel = true
"#,
    )
    .unwrap();
    app.install(Observability::new());
    app.get("/ping", |_r: Request| async { Html("pong") });
    let c = TestClient::tracked(app).unwrap();
    assert_eq!(c.get("/ping").await.status_code().as_u16(), 200);
}

/// When feature `elasticsearch` is on, `[observability] elasticsearch = true`
/// is applied; missing URL only warns.
#[cfg(feature = "elasticsearch")]
#[tokio::test]
async fn elasticsearch_bool_from_toml_does_not_panic() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[observability]
elasticsearch = true
"#,
    )
    .unwrap();
    app.install(Observability::new());
    app.get("/ping", |_r: Request| async { Html("pong") });
    let c = TestClient::tracked(app).unwrap();
    assert_eq!(c.get("/ping").await.status_code().as_u16(), 200);
}
