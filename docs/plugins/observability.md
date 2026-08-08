---
title: observability
editLink: false
---

# `observability`

**HTTP metrics, OpenTelemetry, Elasticsearch log shipping** · crate `ruvo-observability` · id `observability`

```bash
cargo add ruvo --features observability,observability-elasticsearch,observability-otel
```

| Feature | What you get |
|---------|-------------|
| `observability` | Prometheus `/metrics`. |
| `observability-elasticsearch` | Ship tracing logs to Elasticsearch. |
| `observability-otel` | OpenTelemetry OTLP export. |

HTTP RED metrics (Prometheus) + optional OpenTelemetry / Elasticsearch logs for Ruvo.

```rust
 app.use_middleware(request_id());
 app.install(
     Observability::new()
         .with_elasticsearch() // ELASTICSEARCH_URL → bulk ship tracing logs
 );
 app.use_middleware(logger());
 ```

 Declarative toggles also work via `[observability]` in `ruvo.toml`
 (`metrics_path`, `otel`, `elasticsearch`) when not set on the builder.

## Usage

Presets already install `request_id` + `logger`. Add metrics/OTel explicitly:

```rust
let mut app = App::api().title("API").version("1.0").into_app();
app.install(Observability::new()); // GET /metrics
// order tip for custom stacks: request_id → Observability → logger
```

Features: `observability-otel`, `observability-elasticsearch`.
