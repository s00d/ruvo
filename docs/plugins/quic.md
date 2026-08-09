---
title: quic
editLink: false
---

# `quic`

**QUIC datagrams BackgroundService helpers for Sova**

| | |
|--|--|
| Crate | [`sova-quic`](https://docs.rs/sova-quic/0.1.1) `0.1.1` |
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

## Quick start

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

## Examples

- `examples/net/quic_udp_echo`

## Related

[`udp`](/plugins/udp)
