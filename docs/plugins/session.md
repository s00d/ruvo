---
title: session
editLink: false
---

# `session`

**Cookie sessions backed by a SessionStore** · crate `sova-session` `0.1.0` · id `session`

```bash
cargo add sova --features session,session-redis,session-sql
```

| Feature | What you get |
|---------|-------------|
| `session` | Cookie sessions + flash (`sova_session`). |
| `session-redis` | Persist sessions in Redis via `RedisPool`. |
| `session-sql` | Persist sessions in SQL via `DbPool`. |

Cookie-backed sessions for Sova (Express [`express-session`](https://expressjs.com/en/resources/middleware/session.html)-style).

 Flash helpers ([`Session::flash`], [`Session::take`]) store one-shot values for the next
 request (status messages, validation errors, old form input).

 Persistence is [`SessionStore`]: [`KvSessionStore`], [`SqlSessionStore`], or
 [`RedisSessionStore`]. Logout others/all:
 [`SessionExt::logout_other_sessions`] / [`SessionExt::logout_all_sessions`].

## Usage

Cookie sessions are part of **`App::web()`** and **`App::api()`** (`memory_sessions`). Read/write in handlers — do not reinstall the layer unless you need SQL/Redis:

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

### Production cookies

Defaults are fine for local HTTP (`Secure` off, `SameSite=Lax`, `HttpOnly` on).
In production set `SOVA_ENV=production` or `SESSION_SECURE=1` so the session cookie
gets the `Secure` flag (also applied to the CSRF `XSRF-TOKEN` cookie). Override with
`SessionLayer::secure(false)` / `[session] secure = false` in `sova.toml` if needed.
