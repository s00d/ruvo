**`App::web()`** serves `public/` at `/assets` when the directory exists. Point elsewhere with builders:

```rust
let mut app = App::web()
    .site("App")
    .public_url("https://example.com")
    .assets("public")
    .assets_mount("/assets");
```

Only install `Static::new(...)` yourself when you skipped the web preset. Demo: `cargo run -p static_files`.
