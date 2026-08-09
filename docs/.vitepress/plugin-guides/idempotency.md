**When:** clients retry POST/PUT/PATCH/DELETE with the same `Idempotency-Key`.

**Does:**
- On cache hit → replay status + body + content-type (`X-Idempotency-Replay: true`)
- On miss → run handler; store 2xx buffered bodies (size cap) with TTL (default 24h)

### Example

```rust
use std::sync::Arc;
use sova::{AppStore, Idempotency, KvStore};

let store = AppStore::memory();
app.install(Idempotency::from_store(Arc::clone(&store.inner)));
// Client: Idempotency-Key: <unique>
```

### Notes
- Needs feature `idempotency` (+ `store`)
- Only mutating methods; missing header → pass-through
