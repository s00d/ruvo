---
title: cors
editLink: false
---

# `cors`

**Cross-Origin Resource Sharing headers** · crate `sova-cors` `0.1.0` · id `cors`

```bash
cargo add sova --features cors
```

| Feature | What you get |
|---------|-------------|
| `cors` | CORS middleware (`sova_cors`). |

CORS plugin for Sova (Express [`cors`](https://expressjs.com/en/resources/middleware/cors.html)-style).

## Usage

`App::web()` and `App::api()` already install Cors. You only customize when the default is wrong:

```rust
use sova::prelude::*;
use sova::{Cors, Parser, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::api().title("API").version("1.0").into_app();
    // Replace / tighten the preset Cors if needed:
    app.install(
        Cors::new()
            .origin("https://app.example.com")
            .credentials(true),
    );

    app.get("/ping", || async { "ok" });
    app.run().await
}
```
