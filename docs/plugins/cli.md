---
title: cli
editLink: false
---

# `cli`

**CLI ServerArgs / listen_args for Sova (local dev)** · crate `sovax` · id `cli`

```bash
cargo add sova --features cli
```

| Feature | What you get |
|---------|-------------|
| `cli` | `ServerArgs` and log CLI flags (`sovax`). |


Optional CLI helpers for local development (`--log-level`, file logging).
 This crate pulls in `clap` — enable only when you want argv parsing.

## Usage

`ServerArgs` is the local-dev CLI surface used by presets:

```rust
use sova::prelude::*;
use sova::{Parser, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("App")
        .public_url("http://127.0.0.1:3000");
    app.run().await
}
```

`app.run()` also exposes framework commands (`routes`, `migrate`, `tasks`, …) depending on installed plugins.
