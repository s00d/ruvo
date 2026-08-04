//! QUIC datagrams echo via BackgroundService.

use bytes::Bytes;
use ruvo::{init_tracing, App, Bind, QuicDatagramService, Result};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let mut app = App::new();

    // Certificate files must exist in current working directory.
    // Example: `cert.pem` + `key.pem`.
    let quic_bind: SocketAddr = "127.0.0.1:9999".parse().unwrap();
    let alpn = vec![b"ruvo-quic-udp".to_vec()];

    let handler: ruvo_quic::QuicDatagramHandler = Arc::new(|_peer, data: Vec<u8>, conn| {
        Box::pin(async move {
            let _ = conn.send_datagram(Bytes::from(data));
        })
    });

    let svc = QuicDatagramService::from_pem(
        quic_bind,
        "cert.pem",
        "key.pem",
        alpn,
        true,
        handler,
    )?;

    app.service(svc);
    app.get("/", |_| async { ruvo::Response::text("quic datagrams echo") });

    app.bind(Bind::Port(3011)).serve().await
}

