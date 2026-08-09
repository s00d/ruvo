//! QUIC datagrams echo via BackgroundService.
//!
//! [`QuicDatagramService::install`] attaches TLS to HTTPS as well — no second `.tls(...)`.

use bytes::Bytes;
use sova::prelude::*;
use sova::QuicDatagramService;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();

    // Certificate files must exist in current working directory.
    // Example: `cert.pem` + `key.pem`.
    let quic_bind: SocketAddr = "127.0.0.1:9999".parse().unwrap();
    let alpn = vec![b"sova_quic-udp".to_vec()];

    let handler: sova_quic::QuicDatagramHandler = Arc::new(|_peer, data: Vec<u8>, conn| {
        Box::pin(async move {
            let _ = conn.send_datagram(Bytes::from(data));
        })
    });

    QuicDatagramService::from_pem(quic_bind, "cert.pem", "key.pem", alpn, true, handler)?
        .install(&mut app);

    app.get("/", || async { "quic datagrams echo" });
    app.listen(3011).await
}
