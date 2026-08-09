---
title: ws
editLink: false
---

# `ws`

**WebSocket hub, origin allowlist, max message size**

| | |
|--|--|
| Crate | [`sova-ws`](https://docs.rs/sova-ws/0.1.1) `0.1.1` |
| Plugin id | `ws` |
| Category | Realtime |

## Install

```bash
cargo add sova --features ws
```

## Features

| Feature | What you get |
|---------|-------------|
| `ws` | WebSocket upgrade + rooms hub (`app.ws`). |

## Overview

**When:** WebSocket hubs (chat, live feeds).

**Does:**
- `app.install(Ws::new())` + `app.ws("/ws", handler)`
- Rooms hub, origin allowlist, max message size
- `session.join` / `hub().broadcast`

### Example

```rust
app.install(Ws::new().origins(["https://example.com"]));
app.ws("/ws", |mut session| async move {
    let _room = session.join("chat");
    while let Some(Ok(msg)) = session.recv().await { /* … */ }
});
```

## Quick start

Install Ws on the **web preset**, keep modules/static/meta from the stack:

```rust
use sova::prelude::*;
use sova::{Html, Message, Parser, ServerArgs, Ws, WsRouteExt};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Chat")
        .public_url("http://127.0.0.1:3000");
    app.install(Ws::new());

    app.get("/", |_| async {
        Html("<h1>Chat</h1><p>connect to /ws</p>")
    });

    app.ws("/ws", |mut session| async move {
        let _room = session.join("chat");
        while let Some(Ok(msg)) = session.recv().await {
            if let Message::Text(text) = msg {
                session
                    .hub()
                    .broadcast("chat", Message::Text(text))
                    .await;
            }
        }
    });

    app.run().await
}
```

```bash
cargo run -p ws_chat
```

## Examples

- `examples/realtime/ws_chat`

## Related

[`sse`](/plugins/sse) · [`notifications`](/plugins/notifications)
