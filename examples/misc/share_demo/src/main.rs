//! TCP socket handoff via [`Slot`] + counter via [`Cell`].
//!
//! ```bash
//! cargo run -p share_demo
//! # terminal 2:  nc 127.0.0.1 9090
//! # terminal 3:  curl -X POST http://127.0.0.1:3020/grab
//! ```

use ruvo::extend::{wait_shutdown, BoxFuture, StateMap};
use ruvo::{
    App, BackgroundService, Cell, Error, IntoResponse, Json, Request, Response, Result, Shutdown,
    Slot,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

struct TcpInbox {
    bind: SocketAddr,
    inbox: Slot<TcpStream>,
}

impl BackgroundService for TcpInbox {
    fn name(&self) -> &str {
        "tcp-inbox"
    }

    fn run(self: Box<Self>, _state: Arc<StateMap>, shutdown: Shutdown) -> BoxFuture<()> {
        Box::pin(async move {
            let listener = match TcpListener::bind(self.bind).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!(error = %e, addr = %self.bind, "tcp inbox bind failed");
                    return;
                }
            };
            tracing::info!(%self.bind, "TCP inbox listening — connect then POST /grab");
            loop {
                tokio::select! {
                    _ = wait_shutdown(shutdown.clone()) => break,
                    acc = listener.accept() => {
                        match acc {
                            Ok((stream, peer)) => {
                                tracing::info!(%peer, "accepted → Slot::put");
                                self.inbox.put(stream);
                            }
                            Err(e) => tracing::warn!(error = %e, "accept failed"),
                        }
                    }
                }
            }
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let inbox = Slot::<TcpStream>::new();
    let handed = Cell::new(0u64);

    let mut app = App::new();
    app.state(inbox.clone());
    app.state(handed.clone());
    app.service(TcpInbox {
        bind: "127.0.0.1:9090".parse().unwrap(),
        inbox,
    });

    app.get("/", |req: Request| async move {
        let n = req.state::<Cell<u64>>().get();
        Json(json!({
            "hint": "nc 127.0.0.1 9090  then  curl -X POST http://127.0.0.1:3020/grab",
            "handed_off": n,
        }))
    });

    app.post("/grab", grab);

    tracing::info!("HTTP http://127.0.0.1:3020  TCP inbox 127.0.0.1:9090");
    app.listen(3020).await
}

async fn grab(req: Request) -> Result<Response> {
    let inbox = req.state::<Slot<TcpStream>>();
    let handed = req.state::<Cell<u64>>();

    let mut stream = tokio::time::timeout(std::time::Duration::from_secs(15), inbox.take())
        .await
        .map_err(|_| Error::BadRequest("no socket in slot (connect with nc first)".into()))?;

    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "?".into());

    let mut buf = [0u8; 64];
    let n = tokio::time::timeout(std::time::Duration::from_millis(200), stream.read(&mut buf))
        .await
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or(0);
    let preview = String::from_utf8_lossy(&buf[..n]).into_owned();

    let _ = stream.write_all(b"ruvo share_demo: got you\n").await;
    let _ = stream.shutdown().await;

    handed.update(|c| c + 1);

    Ok(Json(json!({
        "peer": peer,
        "bytes_read": n,
        "preview": preview,
        "handed_off": handed.get(),
    }))
    .into_response())
}
