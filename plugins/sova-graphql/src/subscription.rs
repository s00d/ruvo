//! GraphQL subscriptions over WebSocket (`graphql-transport-ws` / `graphql-ws`).

use crate::context::GraphqlContext;
use async_graphql::http::{
    ClientMessage, WebSocket, WebSocketProtocols as Protocols, WsMessage, ALL_WEBSOCKET_PROTOCOLS,
};
use async_graphql::{Data, Executor};
use futures_util::{SinkExt, StreamExt};
use http::header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL,
    SEC_WEBSOCKET_VERSION, UPGRADE,
};
use http::HeaderMap;
use hyper_util::rt::TokioIo;
use sova_core::{App, Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::protocol::{Role, WebSocketConfig};
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::WebSocketStream;

type WsIo = TokioIo<hyper::upgrade::Upgraded>;

pub(crate) type WsRunner = Arc<
    dyn Fn(
            WebSocketStream<WsIo>,
            GraphqlContext,
            Protocols,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

pub(crate) fn ws_runner<E>(executor: E) -> WsRunner
where
    E: Executor + Send + Sync + 'static,
{
    Arc::new(move |stream, ctx, protocol| {
        let executor = executor.clone();
        Box::pin(run_websocket(stream, executor, ctx, protocol))
    })
}

pub(crate) fn install_subscription_mount(app: &mut App, path: &str, ws: WsRunner) {
    let path = super::server::normalize_path(path);
    let ws_runner = ws;
    app.get(&path, move |req: Request| {
        let ws = Arc::clone(&ws_runner);
        async move { upgrade_graphql_ws(req, ws).await }
    });
}

async fn upgrade_graphql_ws(mut req: Request, ws: WsRunner) -> Response {
    if let Err(res) = validate_ws_headers(&req.headers) {
        return *res;
    }
    let protocol = match parse_protocol(&req.headers) {
        Ok(p) => p,
        Err(res) => return *res,
    };
    let key = match ws_key(&req.headers) {
        Ok(k) => k,
        Err(res) => return *res,
    };
    let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
    let ctx = GraphqlContext::from_request(&req);
    tracing::debug!(
        target: "sova.graphql.ws",
        event = "upgrade",
        protocol = protocol.sec_websocket_protocol(),
        path = %req.path,
    );

    let on_upgrade = match req.on_upgrade() {
        None => {
            return Response::text("Upgrade Required")
                .status(426)
                .header(UPGRADE.as_str(), "websocket");
        }
        Some(Err(res)) => return res,
        Some(Ok(up)) => up,
    };

    tokio::spawn(async move {
        let io = match on_upgrade.upgrade().await {
            Ok((io, _permit)) => io,
            Err(e) => {
                tracing::warn!("graphql ws upgrade failed: {e}");
                return;
            }
        };
        let stream = WebSocketStream::from_raw_socket(
            TokioIo::new(io),
            Role::Server,
            Some(WebSocketConfig::default()),
        )
        .await;
        ws(stream, ctx, protocol).await;
    });

    let mut res = Response::empty()
        .status(101)
        .header(UPGRADE.as_str(), "websocket")
        .header(CONNECTION.as_str(), "upgrade")
        .header(SEC_WEBSOCKET_ACCEPT.as_str(), accept)
        .header(SEC_WEBSOCKET_VERSION.as_str(), "13");
    if req.headers.get(SEC_WEBSOCKET_PROTOCOL).is_some() {
        res = res.header(
            SEC_WEBSOCKET_PROTOCOL.as_str(),
            protocol.sec_websocket_protocol(),
        );
    }
    res
}

async fn run_websocket<E>(
    stream: WebSocketStream<WsIo>,
    executor: E,
    ctx: GraphqlContext,
    protocol: Protocols,
) where
    E: Executor,
{
    let (mut write, read) = stream.split();
    let read = read.filter_map(|msg| async move {
        match msg {
            Ok(Message::Text(text)) => Some(ClientMessage::from_bytes(text.as_bytes())),
            Ok(Message::Binary(bytes)) => Some(ClientMessage::from_bytes(&bytes)),
            _ => None,
        }
    });

    let mut data = Data::default();
    data.insert(ctx);

    let mut gql_ws =
        Box::pin(WebSocket::from_message_stream(executor, read, protocol).connection_data(data));

    while let Some(msg) = gql_ws.next().await {
        match msg {
            WsMessage::Text(text) => {
                if write.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            WsMessage::Close(code, reason) => {
                let _ = write.close().await;
                let _ = (code, reason);
                break;
            }
        }
    }
}

fn validate_ws_headers(headers: &HeaderMap) -> Result<(), Box<Response>> {
    let upgrade_ok = headers
        .get(UPGRADE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    if !upgrade_ok {
        return Err(Box::new(Response::text("Bad Request").status(400)));
    }
    if headers.get(SEC_WEBSOCKET_KEY).is_none() {
        return Err(Box::new(Response::text("Bad Request").status(400)));
    }
    let version_ok = headers
        .get(SEC_WEBSOCKET_VERSION)
        .and_then(|v| v.to_str().ok())
        == Some("13");
    if !version_ok {
        return Err(Box::new(Response::text("Bad Request").status(400)));
    }
    Ok(())
}

fn ws_key(headers: &HeaderMap) -> Result<&str, Box<Response>> {
    headers
        .get(SEC_WEBSOCKET_KEY)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Box::new(Response::text("Bad Request").status(400)))
}

fn parse_protocol(headers: &HeaderMap) -> Result<Protocols, Box<Response>> {
    let Some(raw) = headers
        .get(SEC_WEBSOCKET_PROTOCOL)
        .and_then(|v| v.to_str().ok())
    else {
        return Ok(Protocols::GraphQLWS);
    };
    for part in raw.split(',').map(str::trim) {
        if ALL_WEBSOCKET_PROTOCOLS
            .iter()
            .any(|p| p.eq_ignore_ascii_case(part))
        {
            return part.parse().map_err(|_| {
                Box::new(Response::text("Unsupported Sec-WebSocket-Protocol").status(400))
            });
        }
    }
    Err(Box::new(
        Response::text("Unsupported Sec-WebSocket-Protocol").status(400),
    ))
}
