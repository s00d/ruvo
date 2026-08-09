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

### Config

```toml
[storage]
driver = "local"          # local | memory | s3 | gcs | azure
path = "./storage"        # local
public_url = "https://cdn.example.com"
bucket = "my-bucket"      # s3/gcs/azure
region = "auto"
endpoint = "https://…"    # MinIO / R2
root = "uploads/"
force_path_style = true
```

Env (wins / fills): `SOVA_STORAGE`, `SOVA_STORAGE_PATH`, `SOVA_STORAGE_PUBLIC_URL`, `SOVA_STORAGE_BUCKET` (+ `AWS_BUCKET`), `SOVA_STORAGE_REGION` (+ `AWS_REGION`), `SOVA_STORAGE_ENDPOINT`, `SOVA_STORAGE_ROOT`, `SOVA_STORAGE_FORCE_PATH_STYLE`, `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`, GCS/Azure vars as in crate docs.
