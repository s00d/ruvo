---
title: store
editLink: false
---

# `store`

**KvStore trait + memory / file / sql / redis backends for Sova**

| | |
|--|--|
| Crate | [`sova-store`](https://docs.rs/sova-store/0.1.2) `0.1.2` |
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
- `namespace(store, "sess")` / `AppStore::namespaced` for isolation
- Optional crypto (`store-crypto`)

### Example

```rust
app.install(Db::from_env());
app.install(SharedStore::sql(&app)); // or ::memory() / ::redis(&app)
```

## Quick start

Shared `KvStore` for rate-limit, cache, etc. Soft-wire from Db/Redis when installed:

```rust
use sova::prelude::*;
use sova::{Db, Parser, ServerArgs, SharedStore};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("App")
        .public_url("https://example.com")
        .into_app();

    app.install(Db::from_env());
    app.install(SharedStore::sql(&app)); // or ::redis(&app) / ::memory()

    app.run().await
}
```

Namespaces: `app.try_state::<AppStore>().unwrap().namespaced("sess")`.

Features: `store-sql`, `store-redis`, `store-file`, `store-crypto`.

## Related

[`session`](/plugins/session) · [`redis`](/plugins/redis) · [`rate-limit`](/plugins/rate-limit) · [`csrf`](/plugins/csrf) · [`idempotency`](/plugins/idempotency) · [`response-cache`](/plugins/response-cache)
