**When:** byte KV for sessions, CSRF, rate-limit, cache.

**Does:**
- `KvStore` + `AppStore` / `SharedStore`
- memory / file / sql / redis / redb backends
- `namespace(store, "sess")` / `AppStore::namespaced` for isolation
- Optional crypto (`store-crypto`)

### Example

```rust
app.install(SharedStore::redb("./data/kv.redb")); // or ::memory() / ::sql(&app) / ::redis(&app)
```
