**When:** QUIC datagrams (TLS 1.3) — not HTTP/3 request streams.

**Does:**
- `QuicDatagramService` as a `BackgroundService`
- Client helper `QuicDatagramClient`
- `.install(&mut app)` attaches TLS to HTTPS (`App::use_tls`) so `reload()` updates both

### Example

```rust
QuicDatagramService::from_pem(quic_bind, "cert.pem", "key.pem", alpn, true, handler)?
    .install(&mut app);
app.listen(3011).await?;
```

See [`examples/net/quic_udp_echo`](https://github.com/s00d/sova/tree/master/examples/net/quic_udp_echo).
