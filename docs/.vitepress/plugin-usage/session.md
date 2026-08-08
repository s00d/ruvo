Cookie sessions are part of **`App::web()`** and **`App::api()`** (`memory_sessions`). Read/write in handlers — do not reinstall the layer unless you need SQL/Redis:

```rust
use ruvo::prelude::*;
use ruvo::{Html, Parser, Redirect, Request, ServerArgs, SessionExt};

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

    app.post("/logout", |req: Request| async move {
        req.session().set("user", "guest");
        Redirect::see_other("/")
    });

    app.run().await
}
```

SQL backend (shared `DbPool`):

```rust
let mut app = App::web().site("App").public_url("https://example.com").into_app();
app.install(Db::from_env());
let pool = app.try_state::<DbPool>().expect("db").as_ref().clone();
app.install(SessionLayer::from_store(Arc::new(
    SqlSessionStore::from_db_pool(&pool),
)));
```

Features: `session-sql`, `session-redis`.
