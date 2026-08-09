**When:** byte KV for sessions, CSRF, rate-limit, cache.

**Does:**
- `KvStore` + `AppStore` / `SharedStore`
- memory / file / sql / redis backends
- `namespace(store, "sess")` / `AppStore::namespaced` for isolation
- Optional crypto (`store-crypto`)

### Example

```rust
app.install(Db::from_env());
app.install(SharedStore::sql(&app)); // or ::memory() / ::redis(&app)
```
