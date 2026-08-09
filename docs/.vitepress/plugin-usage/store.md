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
