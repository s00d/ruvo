**When:** throttle by IP, user id, or custom key.

**Does:**
- Fixed-window limits via in-memory or `KvStore` backend
- Presets: `per_minute`, `login`, `forgot`, …
- Key strategies: IP / identity / custom `key_fn`

### Example

```rust
use std::time::Duration;
app.install(RateLimit::new(60, Duration::from_secs(60)));
// or shared store:
app.install(RateLimit::fixed_window(store, 120, Duration::from_secs(60)));
```

### Notes
- Multi-instance: use `store` / `redis` + `fixed_window`
