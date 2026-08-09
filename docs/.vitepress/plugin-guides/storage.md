**When:** object storage (local disk, memory, S3, GCS, Azure).

**Does:**
- `Storage::from_env()?` → `req.storage()`
- `put` / `get` / `delete` (+ upload helper)
- Driver via features + env

### Example

```rust
app.install(Storage::from_env()?);
req.storage().put("avatars/1.png", bytes, PutOpts::default()).await?;
```
