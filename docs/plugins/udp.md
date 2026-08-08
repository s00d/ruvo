---
title: udp
editLink: false
---

# `udp`

**UDP BackgroundService helpers for Sova** · crate `sova-udp` · id `udp`

```bash
cargo add sova --features udp
```

| Feature | What you get |
|---------|-------------|
| `udp` | UDP `BackgroundService` (`sova_udp`). |

UDP listeners as `BackgroundService`.

## Usage

Low-level UDP helpers — not part of web/api presets. See the runnable demo:

```bash
cargo run -p udp_echo
```

Source: `examples/net/udp_echo`.
