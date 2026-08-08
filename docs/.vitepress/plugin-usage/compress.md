Optional response compression on top of a preset:

```rust
let mut app = App::web()
    .site("App")
    .public_url("https://example.com")
    .into_app();
app.install(Compress::new());
app.run().await
```
