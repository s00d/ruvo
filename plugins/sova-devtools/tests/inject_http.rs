//! Integration: inject HTML only; JSON untouched.

use sova_core::{Html, Json, ResponseAssert, TestClient};
use sova_devtools::DevTools;

#[tokio::test]
async fn injects_html_skips_json() {
    let mut app = sova_core::App::new();
    app.use_middleware(sova_core::request_id());
    app.install(DevTools::new().enabled(true));
    app.get("/h", || async { Html("<html><body>hi</body></html>") });
    app.get("/j", || async { Json(serde_json::json!({ "ok": true })) });

    let client = TestClient::new(app).expect("client");
    let html = client.get("/h").await;
    html.assert_status(200);
    let cc = html
        .headers()
        .get("cache-control")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        cc.contains("no-store"),
        "html should disable bfcache, got {cc:?}"
    );
    let body = String::from_utf8_lossy(html.body_bytes().expect("body"));
    assert!(body.contains("sova-devtools"), "{body}");
    assert!(body.contains("<!-- sova_devtools -->"), "{body}");
    assert!(body.contains("bridge.js"), "{body}");
    assert!(!body.contains("app.js"), "{body}");

    let json = client.get("/j").await;
    json.assert_status(200);
    let json_cc = json.headers().get("cache-control");
    assert!(
        json_cc.is_none(),
        "json must not get DevTools cache headers"
    );
    let raw = String::from_utf8_lossy(json.body_bytes().expect("body"));
    assert!(!raw.contains("sova-devtools"), "{raw}");
    assert!(
        raw.contains("\"ok\":true") || raw.contains("\"ok\": true"),
        "{raw}"
    );
}

#[tokio::test]
async fn requests_api_lists_snapshots() {
    let mut app = sova_core::App::new();
    app.install(DevTools::new().enabled(true));
    app.get("/", || async { Html("<html><body>x</body></html>") });
    let client = TestClient::new(app).expect("client");
    let _ = client.get("/").await;
    let list = client.get("/_devtools/requests").await;
    list.assert_status(200);
}

#[tokio::test]
async fn serves_spa_shell() {
    let mut app = sova_core::App::new();
    app.install(DevTools::new().enabled(true));
    let client = TestClient::new(app).expect("client");
    let shell = client.get("/_devtools/app").await;
    shell.assert_status(200);
    let body = String::from_utf8_lossy(shell.body_bytes().expect("body"));
    assert!(body.contains("app.js"), "{body}");
    assert!(body.contains("app.css"), "{body}");
}
