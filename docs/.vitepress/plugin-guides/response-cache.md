**When:** cache public GET 200 responses (lists, marketing pages).

**Does:**
- Skips requests with `Authorization` / `Cookie` unless `cache_private(true)`
- Key = method + path + sorted `Vary` headers
- Sets `X-Cache: HIT|MISS` and `Cache-Control`
- Invalidate via `ResponseCacheHandle::invalidate_prefix` (e.g. from an `EventBus` listener)

### Example

```rust
use std::sync::Arc;
use std::time::Duration;
use sova::{Memory, ResponseCache};

let store = Arc::new(Memory::new());
app.install(ResponseCache::new(store).ttl(Duration::from_secs(60)).vary(&["accept-language"]));
```

### Notes
- Feature `response-cache` (+ `store`)
- Does not replace static file ETag — dynamic routes only
