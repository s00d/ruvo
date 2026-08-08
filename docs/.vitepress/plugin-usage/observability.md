Presets already install `request_id` + `logger`. Add metrics/OTel explicitly:

```rust
let mut app = App::api().title("API").version("1.0").into_app();
app.install(Observability::new()); // GET /metrics
// order tip for custom stacks: request_id → Observability → logger
```

Features: `observability-otel`, `observability-elasticsearch`.
