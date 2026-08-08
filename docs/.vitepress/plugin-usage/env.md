**`App::web()` / `App::api()`** load env via the `env` feature when the preset starts. Prefer `ServerArgs` + `configure()` / `sova.toml` over ad-hoc dotenv calls.

```rust
let args = ServerArgs::parse();
args.init_tracing();

let mut app = App::web()
    .site("App")
    .public_url("https://example.com");
// configure() already ran inside the preset; override path if needed:
// let mut app = App::web()....into_app();
// let _ = app.configure_from_path("sova.toml");

app.run().await
```

See [Configuration](/guide/configuration).
