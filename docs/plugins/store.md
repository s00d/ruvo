---
title: store
editLink: false
---

# `store`

**KvStore trait + memory / file / sql / redis / redb backends for Sova**

| | |
|--|--|
| Crate | [`sova-store`](https://docs.rs/sova-store/0.1.6) `0.1.6` |
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
| `store-redb` | Embedded redb `KvStore` (file, no daemon). |
| `store-redis` | Redis `KvStore` on `RedisPool`. |
| `store-sql` | SQL `KvStore` on `DbPool`. |

## Overview

**When:** byte KV for sessions, CSRF, rate-limit, cache.

**Does:**
- `KvStore` + `AppStore` / `SharedStore`
- memory / file / sql / redis / redb backends
- `namespace(store, "sess")` / `AppStore::namespaced` for isolation
- Optional crypto (`store-crypto`)

### Example

```rust
app.install(SharedStore::redb("./data/kv.redb")); // or ::memory() / ::sql(&app) / ::redis(&app)
```

## Quick start

Shared `KvStore` for rate-limit, cache, session, etc. Soft-wire from Db/Redis or open embedded redb:

```rust
use sova::prelude::*;
use sova::{Parser, ServerArgs, SharedStore};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("App")
        .public_url("https://example.com")
        .into_app();

    // Embedded (no daemon):
    app.install(SharedStore::redb("./data/kv.redb"));
    // Or: SharedStore::memory() / ::sql(&app) after Db / ::redis(&app) after Redis

    app.run().await
}
```

Namespaces: `app.try_state::<AppStore>().unwrap().namespaced("sess")`.

Features: `store-sql`, `store-redis`, `store-redb`, `store-file`, `store-crypto`.

## Examples

- [`examples/misc/redb`](https://github.com/s00d/sova/tree/master/examples/misc/redb)

## Related

[`session`](/plugins/session) · [`redis`](/plugins/redis) · [`rate-limit`](/plugins/rate-limit) · [`csrf`](/plugins/csrf) · [`idempotency`](/plugins/idempotency) · [`response-cache`](/plugins/response-cache)
