---
title: storage
editLink: false
---

# `storage`

**Object storage (local / memory / S3 / GCS / Azure)**

| | |
|--|--|
| Crate | [`sova-storage`](https://docs.rs/sova-storage/0.1.1) `0.1.1` |
| Plugin id | `storage` |
| Category | Data |

## Install

```bash
cargo add sova --features storage
```

## Features

| Feature | What you get |
|---------|-------------|
| `storage` | Object storage (`req.storage()` — local / cloud). |
| `storage-azure` | Azure Blob backend. |
| `storage-gcs` | Google Cloud Storage backend. |
| `storage-memory` | In-memory blob store (tests). |
| `storage-s3` | S3 / R2 / MinIO backend. |

## Overview

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

## Quick start

Object storage for uploads. Add on the **web preset** (multipart feature for files):

```rust
use sova::prelude::*;
use sova::{Parser, Redirect, Request, ServerArgs, Storage, UploadRules};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Uploads")
        .public_url("http://127.0.0.1:3000")
        .into_app();

    app.install(Storage::local("public/uploads"));

    app.post("/avatar", avatar);
    app.run().await
}

async fn avatar(mut req: Request) -> Result<Redirect> {
    let data = req.input().await?;
    let file = data
        .file("avatar")
        .cloned()
        .ok_or_else(|| Error::bad_request("avatar required"))?;
    file.validate(
        &UploadRules::new()
            .max_bytes(2_000_000)
            .extensions(["png", "jpg", "webp"]),
    )?;
    let _stored = req.storage().store(&file, "avatars").await?;
    Ok(Redirect::see_other("/"))
}
```

```bash
cargo run -p upload
cargo run -p storage_demo
```

## Examples

- `examples/misc/storage`
- `examples/web/upload`

## Related

[`static`](/plugins/static)
