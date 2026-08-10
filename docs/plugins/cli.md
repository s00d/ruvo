---
title: cli
editLink: false
---

# `cli`

**CLI ServerArgs / listen_args for Sova (local dev)**

| | |
|--|--|
| Crate | [`sovax`](https://docs.rs/sovax/0.1.2) `0.1.2` |
| Plugin id | `cli` |
| Category | Tooling |

## Install

```bash
cargo install cargo-sovax
# or: cargo run -p sovax -- <cmd>
```

## Overview

**When:** local CLI (`cargo sovax`) — new apps, db migrate, tasks.

**Does:**
- `ServerArgs` / `listen_args` for apps
- Scaffold: `cargo sovax new …`
- Subcommands for db / tasks

### Example

```bash
cargo install cargo-sovax
cargo sovax new blog --web
```

## Quick start

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

## Examples

- [`examples/basic/cli`](https://github.com/s00d/sova/tree/master/examples/basic/cli)

## Related

[`env`](/plugins/env) · [`db`](/plugins/db) · [`tasks`](/plugins/tasks)
