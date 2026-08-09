use sova_core::{App, CheckKind, Error, TestClient};

#[tokio::test]
async fn healthz_always_ok_without_checks() {
    let mut app = App::new();
    app.with_probes();
    app.register_check("boom", |_s| async {
        Err(Error::Internal("should not run".into()))
    });

    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/healthz").await;
    assert_eq!(res.status_code().as_u16(), 200);
    let body = String::from_utf8_lossy(res.body_bytes().unwrap_or(b""));
    assert!(body.contains(r#""status":"ok"#));
}

#[tokio::test]
async fn ready_503_when_ready_check_fails() {
    let mut app = App::new();
    app.register_check("db", |_s| async {
        Err(Error::Internal("connection refused".into()))
    });
    app.register_audit("openapi", |_s| async { Ok(()) });
    app.with_probes();

    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/ready").await;
    assert_eq!(res.status_code().as_u16(), 503);
    let body = String::from_utf8_lossy(res.body_bytes().unwrap_or(b""));
    assert!(body.contains(r#""status":"not_ready"#));
    assert!(body.contains("db"));
    assert!(!body.contains("openapi"));
}

#[tokio::test]
async fn ready_ignores_audit_failures() {
    let mut app = App::new();
    app.register_check("db", |_s| async { Ok(()) });
    app.register_audit("openapi", |_s| async {
        Err(Error::Internal("schema drift".into()))
    });
    app.with_probes();

    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/ready").await;
    assert_eq!(res.status_code().as_u16(), 200);
    let body = String::from_utf8_lossy(res.body_bytes().unwrap_or(b""));
    assert!(body.contains(r#""status":"ok"#));
    assert!(body.contains("db"));
    assert!(!body.contains("openapi"));
}

#[tokio::test]
async fn run_checks_includes_audit_for_cli() {
    let mut app = App::new();
    app.register_check("db", |_s| async { Ok(()) });
    app.register_audit("openapi", |_s| async {
        Err(Error::Internal("bad".into()))
    });

    let server = app.build().unwrap();
    let results = app
        .run_checks(server.state(), &[CheckKind::Ready, CheckKind::Audit])
        .await;
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.name == "db" && r.ok));
    assert!(results.iter().any(|r| r.name == "openapi" && !r.ok));
}

#[tokio::test]
async fn ready_exposes_instance_header() {
    std::env::set_var("SOVA_INSTANCE_ID", "test-instance-1");
    let mut app = App::new();
    app.with_probes();
    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/ready").await;
    assert_eq!(res.status_code().as_u16(), 200);
    assert_eq!(
        res.headers().get("x-sova-instance").and_then(|v| v.to_str().ok()),
        Some("test-instance-1")
    );
    std::env::remove_var("SOVA_INSTANCE_ID");
}

#[tokio::test]
async fn route_redirect_helper() {
    let mut app = App::new();
    app.redirect("/old", "/new", 302);
    app.redirect("/gone", "https://example.com/", 301);

    let c = TestClient::tracked(app).await.unwrap();

    let res = c.get("/old").await;
    assert_eq!(res.status_code().as_u16(), 302);
    assert_eq!(res.headers().get("location").unwrap(), "/new");

    let res = c.get("/gone").await;
    assert_eq!(res.status_code().as_u16(), 301);
    assert_eq!(
        res.headers().get("location").unwrap(),
        "https://example.com/"
    );
}
