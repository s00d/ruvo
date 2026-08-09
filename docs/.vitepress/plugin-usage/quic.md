QUIC datagrams share `Tls` with HTTPS (reload updates both). Not in presets:

```rust
use sova::QuicDatagramService;
// handler: QuicDatagramHandler = Arc::new(|peer, data, conn| Box::pin(async move { … }));

QuicDatagramService::from_pem(quic_bind, "cert.pem", "key.pem", alpn, true, handler)?
    .install(&mut app); // wires QUIC + App::use_tls
app.listen(3011).await?;
```

```bash
cargo run -p quic_udp_echo
```
