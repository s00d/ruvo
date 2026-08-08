---
title: ws
editLink: false
---

# `ws`

**WebSocket hub, origin allowlist, max message size** · crate `ruvo-ws` · id `ws`

```bash
cargo add ruvo --features ws
```

| Feature | What you get |
|---------|-------------|
| `ws` | WebSocket upgrades (`ruvo-ws`). |

WebSocket plugin for Ruvo (HTTP upgrade + rooms hub).

## Usage

Install Ws on the **web preset**, keep modules/static/meta from the stack:

```rust
use ruvo::prelude::*;
use ruvo::{Html, Message, Parser, ServerArgs, Ws, WsRouteExt};

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
