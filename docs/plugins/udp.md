---
title: udp
editLink: false
---

# `udp`

**UDP BackgroundService helpers for Sova**

| | |
|--|--|
| Crate | [`sova-udp`](https://docs.rs/sova-udp/0.1.1) `0.1.1` |
| Plugin id | `udp` |
| Category | Realtime |

## Install

```bash
cargo add sova --features udp
```

## Features

| Feature | What you get |
|---------|-------------|
| `udp` | UDP `BackgroundService` (`UdpService`). |

## Overview

**When:** datagram listeners as background services (not HTTP).

**Does:**
- `UdpService` binds + handler per packet
- Built-in `echo` helper
- Shutdown-aware via `app.service(...)`

### Example

```rust
use sova::{App, UdpService};
use std::net::SocketAddr;

let mut app = App::new();
app.service(UdpService::echo("127.0.0.1:9999".parse()?));
app.listen(3011).await?;
```

## Quick start

Not part of web/api presets — register as a background service:

```rust
use sova::{App, Result, UdpService};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    let addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
    app.service(UdpService::echo(addr));
    app.get("/", |_| async { sova::Response::text("udp echo on :9999") });
    app.listen(3011).await
}
```

```bash
cargo run -p udp_echo
```

## Examples

- [`examples/net/udp_echo`](https://github.com/s00d/sova/tree/master/examples/net/udp_echo)

## Related

[`quic`](/plugins/quic)
