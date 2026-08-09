**When:** browser clients on another origin call your API. Already on `App::api()` / `App::web()`.

**Does:**
- CORS preflight + response headers
- Origin allowlist / mirror / credentials

### Example

```rust
app.install(Cors::new().origins(["https://app.example.com"]).credentials(true));
```
