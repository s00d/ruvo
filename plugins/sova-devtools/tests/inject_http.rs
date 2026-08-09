//! Integration: inject HTML only; JSON untouched.

use sova_core::{Html, Json, Response, ResponseAssert, TestClient};
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
async fn snapshot_has_route_and_encoding() {
    let mut app = sova_core::App::new();
    app.use_middleware(sova_core::request_id());
    app.install(DevTools::new().enabled(true));
    app.get("/page", || async {
        Response::html("<html><body>ok</body></html>").header("content-encoding", "gzip")
    });
    let client = TestClient::new(app).expect("client");
    let page = client.get("/page").await;
    page.assert_status(200);

    let list = client.get("/_devtools/requests").await;
    list.assert_status(200);
    let arr: serde_json::Value =
        serde_json::from_slice(list.body_bytes().expect("body")).expect("json");
    let id = arr
        .as_array()
        .and_then(|a| a.first())
        .and_then(|m| m.get("id"))
        .and_then(|v| v.as_str())
        .expect("snap id")
        .to_string();

    let snap = client.get(&format!("/_devtools/requests/{id}")).await;
    snap.assert_status(200);
    let body: serde_json::Value =
        serde_json::from_slice(snap.body_bytes().expect("body")).expect("json");
    assert_eq!(body["path"], "/page");
    assert_eq!(body["encoding"], "gzip");
    assert!(body.get("route").is_some(), "{body}");
    assert!(body.get("cache").is_some(), "{body}");
}

#[tokio::test]
async fn store_tracing_fills_cache_lines() {
    let _ = sova_core::ensure_tracing();
    let mut app = sova_core::App::new();
    app.use_middleware(sova_core::request_id());
    app.install(DevTools::new().enabled(true));
    app.get("/cache-demo", || async {
        tracing::debug!(
            target: "sova.store",
            op = "get",
            backend = "cache",
            key = "demo:key",
            hit = true,
            duration_ms = 1.5,
            "sova.store"
        );
        Html("<html><body>c</body></html>")
    });
    let client = TestClient::new(app).expect("client");
    let _ = client.get("/cache-demo").await;

    let list = client.get("/_devtools/requests").await;
    let arr: serde_json::Value =
        serde_json::from_slice(list.body_bytes().expect("body")).expect("json");
    let id = arr
        .as_array()
        .and_then(|a| a.first())
        .and_then(|m| m.get("id"))
        .and_then(|v| v.as_str())
        .expect("id")
        .to_string();
    let snap = client.get(&format!("/_devtools/requests/{id}")).await;
    let body: serde_json::Value =
        serde_json::from_slice(snap.body_bytes().expect("body")).expect("json");
    let cache = body["cache"].as_array().expect("cache arr");
    assert!(
        cache.iter().any(|c| c["op"] == "get" && c["key"] == "demo:key"),
        "{body}"
    );
}

#[tokio::test]
async fn tasks_tracing_fills_jobs() {
    let _ = sova_core::ensure_tracing();
    let mut app = sova_core::App::new();
    app.use_middleware(sova_core::request_id());
    app.install(DevTools::new().enabled(true));
    app.get("/job-demo", || async {
        tracing::debug!(
            target: "sova.tasks",
            name = "ping",
            queue = "default",
            id = "t_test",
            status = "enqueued",
            duration_ms = 0.5,
            "sova.tasks enqueue"
        );
        Html("<html><body>j</body></html>")
    });
    let client = TestClient::new(app).expect("client");
    let _ = client.get("/job-demo").await;

    let list = client.get("/_devtools/requests").await;
    let arr: serde_json::Value =
        serde_json::from_slice(list.body_bytes().expect("body")).expect("json");
    let id = arr
        .as_array()
        .and_then(|a| a.first())
        .and_then(|m| m.get("id"))
        .and_then(|v| v.as_str())
        .expect("id")
        .to_string();
    let snap = client.get(&format!("/_devtools/requests/{id}")).await;
    let body: serde_json::Value =
        serde_json::from_slice(snap.body_bytes().expect("body")).expect("json");
    let jobs = body["jobs"].as_array().expect("jobs");
    assert!(
        jobs.iter().any(|j| j["name"] == "ping" && j["status"] == "enqueued"),
        "{body}"
    );
}

