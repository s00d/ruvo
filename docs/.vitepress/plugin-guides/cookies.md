**When:** read `Cookie` header or set cookies on responses.

**Does:**
- Parses cookies into request-local `Cookies`
- `req.cookies().get("name")`
- `Response::cookie(...)` helpers

### Example

```rust
app.install(CookieLayer);
let theme = req.get::<Cookies>().and_then(|c| c.get("theme"));
```

### Notes
- Sessions need cookies; prefer `session` plugin for signed session cookies
