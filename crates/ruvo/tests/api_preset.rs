#![cfg(feature = "api")]

use http::Method;
use ruvo::{App, Request, Response};

#[tokio::test]
async fn api_preset_into_app_routes_and_probes() {
    let mut app = App::api()
        .title("Coverage API")
        .version("0.1")
        .docs_mount("/docs")
        .into_app();
    app.get("/ping", |_r: Request| async { Response::text("pong") });

    let server = app.build().unwrap();

    let ping = server.handle_request(Method::GET, "/ping", "").await;
    assert_eq!(ping.body_bytes(), Some(b"pong".as_slice()));

    let health = server.handle_request(Method::GET, "/healthz", "").await;
    assert_eq!(health.status_code().as_u16(), 200);

    let ready = server.handle_request(Method::GET, "/ready", "").await;
    assert_eq!(ready.status_code().as_u16(), 200);

    let docs = server
        .handle_request(Method::GET, "/docs/openapi.json", "")
        .await;
    assert_eq!(docs.status_code().as_u16(), 200);
    let body = String::from_utf8(docs.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("Coverage API") || body.contains("openapi"), "{body}");
}

#[tokio::test]
async fn api_preset_deref_installs_on_mut() {
    let mut app = App::api().title("Mut").version("2.0");
    // DerefMut triggers ensure_installed
    app.get("/x", |_r: Request| async { Response::text("x") });
    let app = app.into_app();
    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/x", "").await;
    assert_eq!(res.body_bytes(), Some(b"x".as_slice()));
    let health = server.handle_request(Method::GET, "/healthz", "").await;
    assert_eq!(health.status_code().as_u16(), 200);
}
