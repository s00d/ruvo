---
title: quic
editLink: false
---

# `quic`

**QUIC datagrams BackgroundService helpers for Sova** · crate `sova-quic` `0.1.1` · id `quic`

```bash
cargo add sova --features quic-udp
```

| Feature | What you get |
|---------|-------------|
| `quic-udp` | QUIC datagrams (`sova_quic`). |

QUIC datagrams (QUIC + TLS 1.3). No DTLS and no HTTP/3 streams — only
 unreliable/unordered application datagrams.

## Usage

QUIC datagram helpers — not part of web/api presets:

```bash
cargo run -p quic_udp_echo
```

Source: `examples/net/quic_udp_echo`.
