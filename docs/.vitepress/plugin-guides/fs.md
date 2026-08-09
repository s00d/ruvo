# Filesystem (`req.fs()`)

Jail-rooted local files and folders — list, walk, read/write/delete. Not object storage (see [storage](/plugins/storage)).

```toml
[dependencies]
sova = { version = "0.1", features = ["fs"] }
```

```rust
app.install(Fs::new("./data"));
// or config: [fs] root = "./data"  /  SOVA_FS_ROOT

let fs = req.fs();
fs.write("notes/a.txt", b"hi").await?;
let kids = fs.read_dir("notes").await?;
let tree = fs.walk("notes").await?; // depth/entries capped
```

Paths are relative to the jail. Absolute paths and `..` escapes return `Forbidden`. Soft EventBus: `FileWritten` / `FileRemoved` / `DirCreated` (DevTools: feature `devtools-fs`).
