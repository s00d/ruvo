**When:** cookie sessions. Already on `App::web()` (memory).

**Does:**
- Session cookie + `SessionStore` backends
- memory / redis / sql features
- Required by CSRF + Fortify

### Example

```rust
app.install(SessionLayer::sql(&app)); // after Db
// or:
app.install(memory_sessions());
app.install(SessionLayer::redis(&app)); // after Redis
app.install(SessionLayer::from_app_store(&app)); // after SharedStore
```

### Config

```toml
[session]
cookie = "sova_sid"
ttl = "7d"            # duration string
same_site = "lax"     # lax | strict | none
secure = true         # optional bool
```

Env: `SOVA_ENV=production` enables Secure cookies unless overridden; `SESSION_SECURE=true|false` forces the flag.
