---
title: static
editLink: false
---

# `static`

**Serve files from a directory under a mount path** · crate `sova-static` `0.1.0` · id `static`

```bash
cargo add sova --features static-files
```

| Feature | What you get |
|---------|-------------|
| `static-files` | Serve static assets via `sova_static`. |

Static file routes as a regular [`Plugin`] — public `Router::get` + conditional headers.

## Usage

**`App::web()`** serves `public/` at `/assets` when the directory exists. Point elsewhere with builders:

```rust
let mut app = App::web()
    .site("App")
    .public_url("https://example.com")
    .assets("public")
    .assets_mount("/assets");
```

Only install `Static::new(...)` yourself when you skipped the web preset. Demo: `cargo run -p static_files`.
