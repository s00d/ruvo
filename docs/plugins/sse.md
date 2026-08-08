---
title: sse
editLink: false
---

# `sse`

**Server-Sent Events helpers for Ruvo (channels, Last-Event-ID, keep-alive)** · crate `ruvo-sse` · id `sse`

```bash
cargo add ruvo --features sse-feed
```

| Feature | What you get |
|---------|-------------|
| `sse-feed` | SSE channel helpers (`ruvo-sse`). |

SSE channel helpers: fan-out, `Last-Event-ID`, keep-alive.

## Usage

```rust
let mut app = App::web()
    .site("Feed")
    .public_url("http://127.0.0.1:3000");
app.install(Sse::new());
// routes — see examples/realtime/sse and sse_feed
app.run().await
```

```bash
cargo run -p sse
cargo run -p sse_feed
```
