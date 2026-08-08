Shared Redis for store/session/tasks — install beside a preset:

```rust
let mut app = App::api().title("API").version("1.0").into_app();
app.install(Redis::from_env());
```

```bash
cargo run -p redis_demo
```
