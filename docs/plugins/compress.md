---
title: compress
editLink: false
---

# `compress`

**gzip / deflate / brotli response compression** · crate `sova-compress` `0.1.0` · id `compress`

```bash
cargo add sova --features compress
```

| Feature | What you get |
|---------|-------------|
| `compress` | gzip/deflate/brotli (`sova-compress`). |

Response compression for Sova (Express [`compression`](https://expressjs.com/en/resources/middleware/compression.html)-style).

 Supports `br`, `gzip`, and `deflate`. Bodies are buffered then compressed
 (not streamed chunk-by-chunk).

```rust
 app.install(Compress::new().threshold(1024).level(6));
 ```

## Usage

Optional response compression on top of a preset:

```rust
let mut app = App::web()
    .site("App")
    .public_url("https://example.com")
    .into_app();
app.install(Compress::new());
app.run().await
```
