```rust
use sova::store::Memory;
use sova::ResponseCache;
use std::sync::Arc;

app.install(ResponseCache::new(Arc::new(Memory::new())));
// From a handler / event listener:
// req.state::<ResponseCacheHandle>().invalidate_prefix("/api/notes").await;
```
