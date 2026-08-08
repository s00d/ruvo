```rust
let mut app = App::web()
    .site("Feed")
    .public_url("http://127.0.0.1:3000");
app.install(Sse::new());
// routes — see examples/realtime/sse and sse_feed
app.run().await
```

```bash
cargo run -p sse
cargo run -p sse_feed
```
