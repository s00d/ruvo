---
title: store
editLink: false
---

# `store`

**KvStore trait + memory / file / sql / redis backends for Ruvo** · crate `ruvo-store` · id `store`

```bash
cargo add ruvo --features store,store-crypto,store-file,store-redis,store-sql
```

| Feature | What you get |
|---------|-------------|
| `store` | KvStore + Cache (`ruvo-store`). |
| `store-crypto` | XChaCha20-Poly1305 wrapper for KvStore. |
| `store-file` | File-backed KvStore. |
| `store-redis` | Redis KvStore on `RedisPool`. |
| `store-sql` | SQL KvStore on `DbPool`. |

Byte-oriented key-value store for Ruvo plugins (sessions, cache, CSRF, rate-limit).

 Trait is stable (memory + file + sql + redis backends).
 Enable feature `unstable-store` for backwards-compatible feature flags.
 **Not in ruvo-core** — wire with `app.state(store.namespace("sess"))`.

## Usage

Shared `KvStore` for rate-limit, cache, etc. Install beside a preset (cabinet uses SQL on the same `DbPool`):

```rust
use std::sync::Arc;
use ruvo::prelude::*;
use ruvo::{Db, DbPool, Parser, ServerArgs, SharedStore};

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
    let kv = Arc::new(ruvo::store::Sql::from_db_pool(&pool)) as Arc<dyn ruvo::KvStore>;
    app.install(SharedStore::new(Arc::clone(&kv)));

    app.run().await
}
```

Features: `store-sql`, `store-redis`, `store-file`, `store-crypto`.
