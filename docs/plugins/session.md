---
title: session
editLink: false
---

# `session`

**Cookie sessions backed by a SessionStore**

| | |
|--|--|
| Crate | [`sova-session`](https://docs.rs/sova-session/0.1.2) `0.1.2` |
| Plugin id | `session` |
| Category | Auth |

## Install

```bash
cargo add sova --features session
```

## Features

| Feature | What you get |
|---------|-------------|
| `session` | Cookie sessions + flash (`SessionLayer` / `memory_sessions`). |
| `session-redis` | Persist sessions in Redis via `RedisPool`. |
| `session-sql` | Persist sessions in SQL via `DbPool`. |

## Overview

**When:** cookie sessions. Already on `App::web()` (memory).

**Does:**
- Session cookie + `SessionStore` backends
- memory / redis / sql features
- Required by CSRF + Fortify

### Example

```rust
app.install(SessionLayer::from_store(store));
// or helper:
app.install(memory_sessions());
```

## Quick start

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

SQL backend — use `App::new()` (or skip the preset session) and install once:

```rust
let mut app = App::new();
app.install(Db::from_env());
let pool = app.try_state::<DbPool>().expect("db").as_ref().clone();
app.install(SessionLayer::from_store(Arc::new(
    SqlSessionStore::from_db_pool(&pool),
)));
```

Features: `session-sql`, `session-redis`.

## Examples

- `examples/cabinet`
- `examples/web/hackernews`

## Related

[`cookies`](/plugins/cookies) · [`csrf`](/plugins/csrf) · [`store`](/plugins/store) · [`redis`](/plugins/redis) · [`auth`](/plugins/auth)
