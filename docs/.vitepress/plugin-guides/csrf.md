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
