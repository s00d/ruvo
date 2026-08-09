**When:** serve `public/` (or any dir) under a mount path. Already on `App::web()` as `/assets`.

**Does:**
- `mount` + `mount/*path` routes
- `max_age`, `immutable`, index files, dotfile guard

### Example

```rust
app.install(Static::new("/assets", "public").max_age(Duration::from_secs(3600)));
```
