Presets already install `request_id` + `logger`. Add metrics / OTel explicitly:

```rust
use sova::{App, Observability};

let mut app = App::api().title("API").version("1.0").into_app();
app.install(
    Observability::new()
        // .with_elasticsearch()  // needs observability-elasticsearch + ELASTICSEARCH_URL
);
// scrape: GET /metrics
// custom stacks: request_id → Observability → logger
```

Features: `observability`, `observability-otel`, `observability-elasticsearch`.
