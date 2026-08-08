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
