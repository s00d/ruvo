Object storage for uploads. Add on the **web preset** (multipart feature for files):

```rust
use ruvo::prelude::*;
use ruvo::{Parser, Redirect, Request, ServerArgs, Storage, UploadRules};

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
