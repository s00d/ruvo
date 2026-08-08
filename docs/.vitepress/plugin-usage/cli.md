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
