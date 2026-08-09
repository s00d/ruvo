**When:** Prometheus RED metrics, optional OpenTelemetry / Elasticsearch logs.

**Does:**
- `GET /metrics` (configurable path)
- HTTP latency/error middleware
- Optional OTel + ES bulk shipping

### Example

```rust
app.install(Observability::new().with_elasticsearch());
```


### Notes
- Typical order: `request_id` → Observability → `logger`

### Config

```toml
[observability]
metrics_path = "/metrics"
otel = true
elasticsearch = true
```

Env: `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME` / `SOVA_SERVICE_NAME`, `ELASTICSEARCH_URL`, `ELASTICSEARCH_USERNAME` / `PASSWORD` / `API_KEY`, `ELASTICSEARCH_INDEX`, `ELASTICSEARCH_BATCH`, `ELASTICSEARCH_FLUSH_MS`.
