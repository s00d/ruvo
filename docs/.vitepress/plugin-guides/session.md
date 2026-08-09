**When:** cookie sessions. Already on `App::web()` (memory).

**Does:**
- Session cookie + `SessionStore` backends
- memory / redis / sql features
- Required by CSRF + Fortify

### Example

```rust
app.install(SessionLayer::from_store(store));
// or helper:
app.install(memory_sessions());
```
