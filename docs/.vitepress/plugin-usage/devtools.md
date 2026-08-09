Debug-only toolbar for **HTML** pages. Full walkthrough: [DevTools guide](/guide/devtools).

```rust
use sova::{App, DevTools, Mail, Parser, ServerArgs};

#[tokio::main]
async fn main() -> sova::Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("App")
        .public_url("http://127.0.0.1:3000")
        .into_app();

    app.install(Mail::from_env()); // optional — Mail tab
    app.install(DevTools::new());  // on in debug; off in --release

    app.get("/", || async { sova::Html("<html><body><h1>hi</h1></body></html>") });
    app.run().await
}
```

```bash
cargo run -p devtools_demo
# http://127.0.0.1:3030/ — click the bottom bar
```

Open Timeline, click another link — SSE updates the list. Mail / HTTP tabs fill after those demo actions.

**Production:** `cargo build --release` keeps DevTools disabled (even with `.enabled(true)`). Use `SOVA_DEVTOOLS=1` only as an ops escape hatch.
