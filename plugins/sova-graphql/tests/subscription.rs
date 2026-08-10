//! GraphQL subscription over WebSocket.

use async_graphql::{Context, EmptyMutation, Object, Schema, Subscription};
use futures_util::{SinkExt, Stream, StreamExt};
use http::header::SEC_WEBSOCKET_PROTOCOL;
use http::HeaderValue;
use sova_core::extend::Bind;
use sova_graphql::GraphQl;
use std::time::Duration;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
};

struct Query;

#[Object]
impl Query {
    async fn ping(&self) -> &str {
        "pong"
    }
}

struct Sub;

#[Subscription]
impl Sub {
    async fn ticks(&self, _ctx: &Context<'_>) -> impl Stream<Item = i32> {
        futures_util::stream::iter([1, 2, 3])
    }
}

#[tokio::test]
async fn subscription_ws_delivers_events() {
    let schema = Schema::build(Query, EmptyMutation, Sub).finish();
    let mut app = sova_core::App::new();
    app.install(
        GraphQl::server(schema)
            .path("/graphql")
            .graphiql(false)
            .subscriptions("/graphql/ws"),
    );

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        app.bind(Bind::Listener(listener))
            .shutdown(async move {
                let _ = rx.await;
            })
            .serve()
            .await
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let url = format!("ws://{addr}/graphql/ws");
    let mut request = url.into_client_request().unwrap();
    request.headers_mut().insert(
        SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static("graphql-transport-ws"),
    );
    let (mut ws, _) = connect_async(request).await.expect("ws connect");
    ws.send(Message::Text(
        r#"{"type":"connection_init","payload":{}}"#.into(),
    ))
    .await
    .unwrap();

    let ack = tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("timeout")
        .expect("stream")
        .expect("msg");
    assert!(ack.to_text().unwrap().contains("connection_ack"));

    ws.send(Message::Text(
        r#"{"id":"1","type":"subscribe","payload":{"query":"subscription { ticks }"}}"#.into(),
    ))
    .await
    .unwrap();

    let mut seen = 0;
    for _ in 0..6 {
        let msg = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .expect("timeout")
            .expect("stream")
            .expect("msg");
        let text = msg.to_text().unwrap();
        if text.contains("\"ticks\"") {
            seen += 1;
            break;
        }
    }
    assert!(seen >= 1, "expected subscription payload");

    let _ = tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .expect("server stop")
        .expect("join");
}
