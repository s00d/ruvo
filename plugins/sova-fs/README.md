# sova-fs

Local filesystem access for [Sova](https://github.com/s00d/sova) with a jail root.

```rust
use sova::prelude::*;
use sova::{Fs, FsExt};

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.install(Fs::new("./data"));

    app.get("/notes", |req: Request| async move {
        let entries = req.fs().read_dir("notes").await?;
        Ok::<_, Error>(format!("{entries:?}"))
    });

    app.listen(3000).await
}
```

Config: `[fs] root = "./data"` or `SOVA_FS_ROOT`. Paths are relative to the jail; `..` / absolute escapes return `Forbidden`.
