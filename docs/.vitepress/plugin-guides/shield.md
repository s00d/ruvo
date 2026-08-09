**When:** baseline browser security headers (helmet-style). Already on `App::web()`.

**Does:**
- Sets `X-Frame-Options`, `X-Content-Type-Options`, `Referrer-Policy`, COOP/CORP, etc.
- Optional CSP via builder
- HSTS stays on TLS (`sova_core::Tls`), not here

### Example

```rust
app.install(Shield::new().frame("DENY"));
```

### Notes
- Install **once** — duplicate `shield` id fails at build
- `App::web()` already installs Shield; do not reinstall
