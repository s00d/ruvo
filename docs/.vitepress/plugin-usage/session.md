Cookie sessions are part of **`App::web()`** and **`App::api()`** (`memory_sessions`). Read/write in handlers — do **not** install a second `SessionLayer` on top of a preset (duplicate `session` id fails at `build`):

```rust
use sova::prelude::*;
use sova::{Html, Parser, Redirect, Request, ServerArgs, SessionExt};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Demo")
        .public_url("http://127.0.0.1:3000");

    app.get("/", |req: Request| async move {
        let user = req.session().get_or("user", "guest");
        Html(format!("<p>hello, {user}</p>"))
    });

    app.post("/login", |req: Request| async move {
        req.session().set("user", "ada");
        Redirect::see_other("/")
    });

    app.run().await
}
```

SQL backend — install Db first, then soft-wire the pool:

```rust
let mut app = App::new();
app.install(Db::from_env());
app.install(SessionLayer::sql(&app));
```

Redis:

```rust
app.install(Redis::from_env());
app.install(SessionLayer::redis(&app));
```

Or reuse [`SharedStore`](/plugins/store):

```rust
app.install(SharedStore::memory()); // or ::sql(&app) / ::redis(&app)
app.install(SessionLayer::from_app_store(&app));
```
