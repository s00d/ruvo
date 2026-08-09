QUIC datagrams share `Tls` with HTTPS (reload updates both). Not in presets:

```rust
use sova::{QuicDatagramService, Tls};
// handler: QuicDatagramHandler = Arc::new(|peer, data, conn| Box::pin(async move { … }));

app.service(QuicDatagramService::from_tls(
    quic_bind, tls.clone(), alpn, true, handler,
)?);
app.bind("0.0.0.0:3011").tls(tls)?.run().await?;
```

```bash
cargo run -p quic_udp_echo
```
