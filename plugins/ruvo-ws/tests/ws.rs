//! WebSocket plugin tests.

use http::Method;
use ruvo_core::{App,  Plugin, Request};
use ruvo_core::extend::Bind;
use ruvo_ws::{origin_allowed, Message, Ws, WsRouteExt};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

#[test]
fn origin_allowlist_unit_cases() {
    let mut headers = http::HeaderMap::new();
    headers.insert("origin", "https://app.test".parse().unwrap());
    assert!(origin_allowed(&headers, &["https://app.test".to_string()]));
    assert!(!origin_allowed(&headers, &["https://other.test".to_string()]));
}

#[tokio::test]
async fn origin_rejected_before_upgrade() {
    let mut app = App::new();
    Ws::new()
        .origins(["https://allowed.test"])
        .install(&mut app);
    app.ws("/ws", |_| async {});

    let req = Request::builder()
        .method(Method::GET)
        .path("/ws")
        .header("origin", "https://evil.test")
        .header("upgrade", "websocket")
        .header("connection", "Upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .build();

    let res = app.handle(req).await;
    assert_eq!(res.status_code().as_u16(), 403);
}

#[tokio::test]
async fn missing_upgrade_returns_426() {
    let mut app = App::new();
    Ws::new().install(&mut app);
    app.ws("/ws", |_| async {});

    let req = Request::builder()
        .method(Method::GET)
        .path("/ws")
        .header("upgrade", "websocket")
        .header("connection", "Upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .build();

    let res = app.handle(req).await;
    assert_eq!(res.status_code().as_u16(), 426);
}

#[tokio::test]
async fn websocket_upgrade_smoke() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;

    let mut app = App::new();
    Ws::new().install(&mut app);
    app.ws("/ws", |mut ws| async move {
        let _room = ws.join("chat");
        while let Some(Ok(msg)) = ws.recv().await {
            if let Message::Text(text) = msg {
                ws.hub()
                    .broadcast("chat", Message::Text(format!("echo:{text}").into()))
                    .await;
            }
        }
    });

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { app.bind(Bind::Listener(listener)).serve().await });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let url = format!("ws://{addr}/ws");
    let req = url.into_client_request().unwrap();
    let (mut client, _) = connect_async(req).await.expect("websocket handshake");

    client
        .send(Message::Text("hi".into()))
        .await
        .expect("send");
    let got = tokio::time::timeout(std::time::Duration::from_secs(2), client.next())
        .await
        .expect("timeout")
        .expect("stream")
        .expect("frame");
    assert_eq!(got, Message::Text("echo:hi".into()));

    drop(client);
    server.abort();
}
