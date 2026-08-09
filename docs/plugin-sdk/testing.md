---
title: Testing
editLink: false
---

# Testing

TestClient, tracing hooks, feature matrix tips.

> Author guide — edit `docs/.vitepress/plugin-sdk-guides/testing.md`, then `pnpm docs:generate`.

### Enable testing APIs

```toml
[dev-dependencies]
sova-core = { version = "…", features = ["testing"] }
# or sova = { …, features = ["testing"] }
```

Gives `TestClient`, `ResponseAssert`, shutdown helpers.

### Install order in tests

Mirror production: `request_id` MW if you need ids, then plugins, then routes.

```rust
let mut app = App::new();
app.use_middleware(sova_core::request_id());
app.install(MyPlugin::new());
app.get("/x", handler);
let client = TestClient::new(app)?;
let res = client.get("/x").await;
res.assert_status(200);
```

Use `TestClient::boot` when startup hooks (DB connect) must run.

### Assert plugin registration

```rust
assert!(app.has_plugin("my-plugin"));
```

(Build the app the same way as production, or inspect after install before `build`.)

### Tracing hooks in tests

Call `ensure_tracing()` / `LogConfig { filter: "my_target=debug,…".into(), .. }.install()` once so `add_log_event_hook` layers see events. Default filter is often `sova=info` and will drop `debug!` from your plugin targets.

### Feature matrix

CI should `cargo test -p my-plugin` with each meaningful feature combination (`redis`, `sql`, …), matching how facade features gate optional code.

### Doc tests

Keep rustdoc examples `ignore` or behind features if they need a runtime; VitePress guides hold the narrative examples.
