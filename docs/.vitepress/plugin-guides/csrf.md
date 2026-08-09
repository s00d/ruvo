**When:** cookie-session web apps (Laravel-style double-submit). Already on `App::web()`.

**Does:**
- Issues CSRF / XSRF cookies
- Validates mutating requests
- Except paths for APIs / webhooks

### Example

```rust
app.install(Csrf::new().except(["/webhooks/*"]));
```

### Notes
- Needs session middleware

### Config

```toml
[csrf]
field = "csrf"
header = "x-csrf-token"
auto = true
```

Secure `XSRF-TOKEN` cookie follows session Secure rules (`SOVA_ENV` / `SESSION_SECURE`). Builder: `.except([...])`, `.only([...])`, `.xsrf_cookie(false)`.
