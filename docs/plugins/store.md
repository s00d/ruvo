---
title: store
editLink: false
---

# `store`

**KvStore trait + memory / file / sql / redis backends for Sova**

| | |
|--|--|
| Crate | [`sova-store`](https://docs.rs/sova-store/0.1.1) `0.1.1` |
| Plugin id | `store` |
| Category | Data |

## Install

```bash
cargo add sova --features store
```

## Features

| Feature | What you get |
|---------|-------------|
| `store` | `KvStore` + `Cache` (sessions, CSRF, rate-limit, …). |
| `store-crypto` | XChaCha20-Poly1305 wrapper for `KvStore`. |
| `store-file` | File-backed `KvStore`. |
| `store-redis` | Redis `KvStore` on `RedisPool`. |
| `store-sql` | SQL `KvStore` on `DbPool`. |

## Overview

**When:** byte KV for sessions, CSRF, rate-limit, cache.

**Does:**
- `KvStore` + `AppStore` / `SharedStore`
- memory / file / sql / redis backends
- `namespace(store, "sess")` for isolation
- Optional crypto (`store-crypto`)

### Example

```rust
app.state(AppStore::memory());
let sess = AppStore::memory().namespaced("sess");
app.install(SharedStore::new(sess));
```

## Quick start

Shared `KvStore` for rate-limit, cache, etc. Namespace with `sova::store::namespace(store, "sess")` or `AppStore::namespaced`:

```rust
use std::sync::Arc;
use sova::prelude::*;
use sova::{Db, DbPool, Parser, ServerArgs, SharedStore};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("App")
        .public_url("https://example.com")
        .into_app();

    app.install(Db::from_env());
    let pool = app.try_state::<DbPool>().expect("db").as_ref().clone();
    let kv = Arc::new(sova::store::Sql::from_db_pool(&pool)) as Arc<dyn sova::KvStore>;
    let sess = sova::store::namespace(Arc::clone(&kv), "sess");
    app.install(SharedStore::new(sess));

    app.run().await
}
```

Features: `store-sql`, `store-redis`, `store-file`, `store-crypto`.

## Related

[`session`](/plugins/session) · [`redis`](/plugins/redis) · [`rate-limit`](/plugins/rate-limit) · [`csrf`](/plugins/csrf)
