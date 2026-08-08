---
title: cli
editLink: false
---

# `cli`

**CLI ServerArgs / listen_args for Sova (local dev)** · crate `sovax` `0.1.0` · id `cli`

Optional CLI helpers for local development (`--log-level`, file logging).
 This crate pulls in `clap` — enable only when you want argv parsing.

 Project scaffolding (`cargo sovax new` / `dev` / `db`) is the separate
 binary crate `cargo-sovax`, not this library.

## Usage

> **Not** the project scaffolder. For `cargo sovax new` / `dev` / `db`, see [cargo-sovax](/guide/cargo-sovax).

`ServerArgs` is the local-dev CLI surface used by presets (crate **`sovax`**, feature `cli`):

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
