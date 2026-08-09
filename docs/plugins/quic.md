---
title: quic
editLink: false
---

# `quic`

**QUIC datagrams BackgroundService helpers for Sova**

| | |
|--|--|
| Crate | [`sova-quic`](https://docs.rs/sova-quic/0.1.2) `0.1.2` |
| Plugin id | `quic` |
| Category | Realtime |

## Install

```bash
cargo add sova --features quic-udp
```

## Features

| Feature | What you get |
|---------|-------------|
| `quic-udp` | QUIC datagrams (`QuicDatagramService`). |

## Overview

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

## Quick start

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

## Examples

- [`examples/net/quic_udp_echo`](https://github.com/s00d/sova/tree/master/examples/net/quic_udp_echo)

## Related

[`udp`](/plugins/udp) · [`acme`](/plugins/acme)
