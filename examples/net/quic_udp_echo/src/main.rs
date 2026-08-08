//! QUIC datagrams echo via BackgroundService.
//!
//! One [`Tls`] is cloned into QUIC and HTTPS so `reload()` updates both.

use bytes::Bytes;
use sova::prelude::*;
use sova::{QuicDatagramService, Tls};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();

    // Certificate files must exist in current working directory.
    // Example: `cert.pem` + `key.pem`.
    let tls = Tls::from_pem("cert.pem", "key.pem")?;

    let quic_bind: SocketAddr = "127.0.0.1:9999".parse().unwrap();
    let alpn = vec![b"sova_quic-udp".to_vec()];

    let handler: sova_quic::QuicDatagramHandler = Arc::new(|_peer, data: Vec<u8>, conn| {
        Box::pin(async move {
            let _ = conn.send_datagram(Bytes::from(data));
        })
    });

    app.service(QuicDatagramService::from_tls(
        quic_bind,
        tls.clone(),
        alpn,
        true,
        handler,
    )?);
    app.get("/", || async { "quic datagrams echo" });

    app.bind("0.0.0.0:3011").tls(tls)?.run().await
}
