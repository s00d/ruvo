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
