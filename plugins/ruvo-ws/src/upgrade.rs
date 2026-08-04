//! WebSocket handshake and session I/O.

use std::future::Future;

use futures_util::{SinkExt, StreamExt};
use http::header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use http::HeaderMap;
use hyper_util::rt::TokioIo;
use ruvo_core::{Request, Response, UpgradePermit};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::{Role, WebSocketConfig};
use tokio_tungstenite::tungstenite::{self, Error as WsError, Message};
use tokio_tungstenite::WebSocketStream;

use crate::hub::{Hub, RoomHandle};
use crate::WsShared;

type WsIo = TokioIo<hyper::upgrade::Upgraded>;

/// Active WebSocket connection passed to route handlers.
pub struct WsSession {
    read: futures_util::stream::SplitStream<WebSocketStream<WsIo>>,
    out_tx: mpsc::UnboundedSender<Message>,
    _write_task: tokio::task::JoinHandle<()>,
    hub: Hub,
    _permit: UpgradePermit,
    rooms: Vec<RoomHandle>,
}

impl WsSession {
    pub(crate) fn new(
        stream: WebSocketStream<WsIo>,
        hub: Hub,
        permit: UpgradePermit,
    ) -> Self {
        let (mut write, read) = stream.split();
        let (out_tx, mut out_rx) = mpsc::unbounded_channel();
        let write_task = tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if write.send(msg).await.is_err() {
                    break;
                }
            }
        });
        Self {
            read,
            out_tx,
            _write_task: write_task,
            hub,
            _permit: permit,
            rooms: Vec::new(),
        }
    }

    pub fn hub(&self) -> &Hub {
        &self.hub
    }

    pub async fn recv(&mut self) -> Option<Result<Message, WsError>> {
        self.read.next().await
    }

    pub async fn send(&self, msg: Message) -> Result<(), WsError> {
        self.out_tx
            .send(msg)
            .map_err(|_| WsError::ConnectionClosed)
    }

    pub fn join(&mut self, room: impl Into<String>) -> RoomHandle {
        let handle = self.hub.register(&room.into(), self.out_tx.clone()).1;
        self.rooms.push(handle.clone());
        handle
    }

    pub fn leave(&mut self, room: &str) {
        self.rooms.retain(|h| h.room() != room);
    }
}

/// Check `Origin` against an allowlist. Empty allowlist → allow all (dev default).
pub fn origin_allowed(headers: &HeaderMap, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    let Some(origin) = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    allowed.iter().any(|o| o == origin)
}

fn validate_ws_headers(
    headers: &HeaderMap,
) -> Result<(), Box<Response>> {
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

/// Perform a WebSocket upgrade from a route handler.
pub async fn upgrade_ws<F, Fut>(
    mut req: Request,
    handler: F,
) -> Result<Response, Response>
where
    F: FnOnce(WsSession) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let shared = req
        .try_state::<WsShared>()
        .ok_or_else(|| Response::text("WebSocket plugin not installed").status(500))?;

    if !origin_allowed(&req.headers, &shared.config.origins) {
        return Err(Response::text("Forbidden").status(403));
    }

    validate_ws_headers(&req.headers).map_err(|b| *b)?;
    let key = ws_key(&req.headers).map_err(|b| *b)?;
    let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());

    let on_upgrade = match req.on_upgrade() {
        None => {
            return Err(Response::text("Upgrade Required")
                .status(426)
                .header(UPGRADE.as_str(), "websocket"));
        }
        Some(Err(res)) => return Err(res),
        Some(Ok(up)) => up,
    };

    let hub = shared.hub.clone();
    let max_message_size = shared.config.max_message_size;
    tokio::spawn(async move {
        let (io, permit) = match on_upgrade.upgrade().await {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!("websocket upgrade failed: {err}");
                return;
            }
        };

        let mut ws_config = WebSocketConfig::default();
        ws_config.max_message_size = max_message_size;
        let stream =
            WebSocketStream::from_raw_socket(TokioIo::new(io), Role::Server, Some(ws_config)).await;
        let session = WsSession::new(stream, hub, permit);
        handler(session).await;
    });

    Ok(Response::empty()
        .status(101)
        .header(UPGRADE.as_str(), "websocket")
        .header(CONNECTION.as_str(), "upgrade")
        .header(SEC_WEBSOCKET_ACCEPT.as_str(), accept)
        .header(SEC_WEBSOCKET_VERSION.as_str(), "13"))
}
