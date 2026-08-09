---
title: cors
editLink: false
---

# `cors`

**Cross-Origin Resource Sharing headers**

| | |
|--|--|
| Crate | [`sova-cors`](https://docs.rs/sova-cors/0.1.1) `0.1.1` |
| Plugin id | `cors` |
| Category | HTTP |

## Install

```bash
cargo add sova --features cors
```

## Features

| Feature | What you get |
|---------|-------------|
| `cors` | Cross-origin headers / preflight (`Cors`). |

## Overview

**When:** browser clients on another origin call your API. Already on `App::api()` / `App::web()`.

**Does:**
- CORS preflight + response headers
- Origin allowlist / mirror / credentials

### Example

```rust
app.install(Cors::new().origins(["https://app.example.com"]).credentials(true));
```

## Quick start

`App::web()` and `App::api()` already install Cors. **Do not** `install(Cors::…)` again — duplicate plugin ids fail at `build`. Customize with an explicit stack:

```rust
use sova::prelude::*;
use sova::{Cors, Parser, ServerArgs, memory_sessions};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::new();
    app.use_middleware(request_id());
    app.use_middleware(logger());
    app.install(
        Cors::new()
            .origin("https://app.example.com")
            .credentials(true),
    );
    app.install(memory_sessions());
    // … OpenApi / routes …

    app.get("/ping", || async { "ok" });
    app.run().await
}
```

## Related

[`shield`](/plugins/shield)
