---
title: observability
editLink: false
---

# `observability`

**HTTP metrics, OpenTelemetry, Elasticsearch log shipping** · crate `sova-observability` `0.1.1` · id `observability`

```bash
cargo add sova --features observability,observability-elasticsearch,observability-otel
```

| Feature | What you get |
|---------|-------------|
| `observability` | Prometheus `/metrics`. |
| `observability-elasticsearch` | Ship tracing logs to Elasticsearch. |
| `observability-otel` | OpenTelemetry OTLP export. |

HTTP RED metrics (Prometheus) + optional OpenTelemetry / Elasticsearch logs for Sova.

```rust
 app.use_middleware(request_id());
 app.install(
     Observability::new()
         .with_elasticsearch() // ELASTICSEARCH_URL → bulk ship tracing logs
 );
 app.use_middleware(logger());
 ```

 Declarative toggles also work via `[observability]` in `sova.toml`
 (`metrics_path`, `otel`, `elasticsearch`) when not set on the builder.

## Usage

Presets already install `request_id` + `logger`. Add metrics/OTel explicitly:

```rust
let mut app = App::api().title("API").version("1.0").into_app();
app.install(Observability::new()); // GET /metrics
// order tip for custom stacks: request_id → Observability → logger
```

Features: `observability-otel`, `observability-elasticsearch`.
