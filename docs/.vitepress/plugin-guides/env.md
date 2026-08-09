**When:** load `.env` / cascade env files before config.

**Does:**
- dotenvy cascade for Sova apps
- Safe to install early in `main`

### Example

```rust
app.install(Env::default());
```
