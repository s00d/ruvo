**When:** QUIC datagrams (TLS 1.3) — not HTTP/3 request streams.

**Does:**
- `QuicDatagramService::from_tls` as a `BackgroundService`
- Client helper `QuicDatagramClient`
- Share one `Tls` with HTTPS so `reload()` updates both

### Example

```rust
app.service(QuicDatagramService::from_tls(
    quic_bind, tls.clone(), alpn, true, handler,
)?);
```

See `examples/net/quic_udp_echo`.
