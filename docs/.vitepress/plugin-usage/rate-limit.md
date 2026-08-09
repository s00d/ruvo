In-memory limiter needs no store. Shared (multi-process) uses AppStore:

```rust
use std::time::Duration;

app.install(Db::from_env());
app.install(SharedStore::sql(&app)); // or ::redb("./data/kv.redb") / ::redis(&app) / ::memory()
app.install(
    RateLimit::shared(&app, 120, Duration::from_secs(60)).key(RateLimitKey::Identity),
);
```

Or pass a KvStore explicitly: `RateLimit::fixed_window(store, 120, Duration::from_secs(60))`.
