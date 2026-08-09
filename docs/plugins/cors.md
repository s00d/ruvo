---
title: cors
editLink: false
---

# `cors`

**Cross-Origin Resource Sharing headers** · crate `sova-cors` `0.1.1` · id `cors`

```bash
cargo add sova --features cors
```

| Feature | What you get |
|---------|-------------|
| `cors` | CORS middleware (`sova_cors`). |

CORS plugin for Sova (Express [`cors`](https://expressjs.com/en/resources/middleware/cors.html)-style).

## Usage

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
