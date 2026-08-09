**When:** byte KV for sessions, CSRF, rate-limit, cache.

**Does:**
- `KvStore` + `AppStore` / `SharedStore`
- memory / file / sql / redis backends
- `namespace(store, "sess")` for isolation
- Optional crypto (`store-crypto`)

### Example

```rust
app.state(AppStore::memory());
let sess = AppStore::memory().namespaced("sess");
app.install(SharedStore::new(sess));
```
