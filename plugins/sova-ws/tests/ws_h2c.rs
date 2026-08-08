use bytes::Bytes;
use http::Method;
use http_body_util::Empty;
use hyper::Request as HyperRequest;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use sova_core::{App,  Plugin};
use sova_core::extend::Bind;
use sova_ws::{Ws, WsRouteExt};

#[tokio::test]
async fn websocket_upgrade_over_h2c_returns_426() {
    let mut app = App::new();
    Ws::new().install(&mut app);
    app.ws("/ws", |_| async {});

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        app.bind(Bind::Listener(listener)).serve().await
    });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    // Prior-knowledge h2c: client speaks HTTP/2 immediately on cleartext.
    let client = Client::builder(TokioExecutor::new())
        .http2_only(true)
        .build_http();

    let uri = format!("http://{}/ws", addr);
    let req = HyperRequest::builder()
        .method(Method::GET)
        .uri(uri)
        .header("upgrade", "websocket")
        .header("connection", "Upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .body(Empty::<Bytes>::new())
        .expect("request");

    let resp = client.request(req).await.expect("h2c request");
    let status = resp.status().as_u16();
    assert!(
        matches!(status, 400 | 426),
        "expected 400/426 on websocket upgrade over h2c, got {status}"
    );
    assert!(
        resp.headers().get(http::header::UPGRADE).is_none(),
        "HTTP/2 hop-by-hop Upgrade header must be stripped"
    );

    server.abort();
}

