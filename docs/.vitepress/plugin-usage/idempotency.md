```rust
use sova::{AppStore, Idempotency};
use std::sync::Arc;

let kv = AppStore::memory();
app.install(Idempotency::from_store(Arc::clone(&kv.inner)).ttl(std::time::Duration::from_secs(3600)));
```
