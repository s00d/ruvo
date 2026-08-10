---
title: fs
editLink: false
---

# `fs`

**Local filesystem with jail root (async CRUD + walk)**

| | |
|--|--|
| Crate | [`sova-fs`](https://docs.rs/sova-fs/0.1.0) `0.1.0` |
| Plugin id | `fs` |
| Category | Storage |

## Install

```bash
cargo add sova --features fs
```

## Features

| Feature | What you get |
|---------|-------------|
| `fs` | Local filesystem jail (`req.fs()` — CRUD + walk). |

## Overview

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

## Examples

- [`examples/misc/fs_demo`](https://github.com/s00d/sova/tree/master/examples/misc/fs_demo)

## Related

[`storage`](/plugins/storage) · [`static`](/plugins/static)
