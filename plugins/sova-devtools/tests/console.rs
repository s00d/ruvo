//! DevTools HTTP console action.

use sova_core::{App, AppDispatch, ResponseAssert, TestClient};
use sova_devtools::DevTools;

#[tokio::test]
async fn http_console_gets_app_route() {
    let mut app = App::new();
    app.state(AppDispatch::default());
    app.install(DevTools::new().enabled(true).console(true));
    app.get("/ping", || async { sova_core::Response::text("pong") });

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/_devtools/actions/http")
        .header("content-type", "application/json")
        .body(r#"{"method":"GET","path":"/ping"}"#)
        .await;
    res.assert_status(200);
    let v = res.json_value();
    assert!(v["ok"].as_bool().unwrap_or(false));
    let result = &v["result"];
    assert_eq!(result["status"], 200);
    assert!(result["body"].as_str().unwrap_or("").contains("pong"));
}

#[tokio::test]
async fn http_console_blocks_devtools_paths() {
    let mut app = App::new();
    app.state(AppDispatch::default());
    app.install(DevTools::new().enabled(true).console(true));

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/_devtools/actions/http")
        .header("content-type", "application/json")
        .body(r#"{"method":"GET","path":"/_devtools/config"}"#)
        .await;
    res.assert_status(400);
    assert!(!res.json_value()["ok"].as_bool().unwrap_or(true));
}

#[cfg(feature = "console-redis")]
#[tokio::test]
async fn redis_console_rejects_flushall() {
    let mut app = App::new();
    app.state(AppDispatch::default());
    app.install(
        DevTools::new()
            .enabled(true)
            .console(true)
            .allow_dangerous(false),
    );

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/_devtools/actions/redis")
        .header("content-type", "application/json")
        .body(r#"{"op":"FLUSHALL"}"#)
        .await;
    res.assert_status(403);
}

#[cfg(feature = "console-session")]
#[tokio::test]
async fn session_console_set_and_list() {
    use sova_session::memory_sessions;

    let mut app = App::new();
    app.install(memory_sessions());
    app.state(AppDispatch::default());
    app.install(DevTools::new().enabled(true).console(true));

    let c = TestClient::tracked(app).await.unwrap();
    let _ = c.get("/").await;

    let res = c
        .post("/_devtools/actions/session")
        .header("content-type", "application/json")
        .body(r#"{"op":"set","key":"role","value":"admin"}"#)
        .await;
    res.assert_status(200);
    let v = res.json_value();
    assert!(v["ok"].as_bool().unwrap_or(false));
    assert_eq!(v["result"]["keys"]["role"].as_str(), Some("admin"));
}

#[cfg(feature = "console-grpc")]
#[tokio::test]
async fn grpc_console_calls_fake() {
    use serde_json::json;
    use sova_grpc::{FakeGrpc, Grpc};

    let fake =
        FakeGrpc::new().stub_json("hello.Greeter/SayHello", json!({ "message": "pong-grpc" }));
    let mut app = App::new();
    app.state(AppDispatch::default());
    app.install(Grpc::fake(fake));
    app.install(DevTools::new().enabled(true).console(true));

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/_devtools/actions/grpc")
        .header("content-type", "application/json")
        .body(r#"{"method":"hello.Greeter/SayHello","body":{"name":"test"}}"#)
        .await;
    res.assert_status(200);
    let v = res.json_value();
    assert!(v["ok"].as_bool().unwrap_or(false));
    assert_eq!(v["result"]["body"]["message"].as_str(), Some("pong-grpc"));
}
