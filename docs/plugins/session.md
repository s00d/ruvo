---
title: session
editLink: false
---

# `session`

**Cookie sessions backed by a SessionStore**

| | |
|--|--|
| Crate | [`sova-session`](https://docs.rs/sova-session/0.1.5) `0.1.5` |
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
app.install(SessionLayer::sql(&app)); // after Db
// or:
app.install(memory_sessions());
app.install(SessionLayer::redis(&app)); // after Redis
app.install(SessionLayer::from_app_store(&app)); // after SharedStore
```

### Config

```toml
[session]
cookie = "sova_sid"
ttl = "7d"            # duration string
same_site = "lax"     # lax | strict | none
secure = true         # optional bool
```

Env: `SOVA_ENV=production` enables Secure cookies unless overridden; `SESSION_SECURE=true|false` forces the flag.

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
app.install(SharedStore::memory()); // or ::redb("./data/kv.redb") / ::sql(&app) / ::redis(&app)
app.install(SessionLayer::from_app_store(&app));
```

## Examples

- `examples/cabinet`
- `examples/web/hackernews`

## Related

[`cookies`](/plugins/cookies) · [`csrf`](/plugins/csrf) · [`store`](/plugins/store) · [`redis`](/plugins/redis) · [`auth`](/plugins/auth)
