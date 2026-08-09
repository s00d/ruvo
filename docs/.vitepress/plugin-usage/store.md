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
