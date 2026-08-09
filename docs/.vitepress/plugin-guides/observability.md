**When:** Prometheus RED metrics, optional OpenTelemetry / Elasticsearch logs.

**Does:**
- `GET /metrics` (configurable path)
- HTTP latency/error middleware
- Optional OTel + ES bulk shipping

### Example

```rust
app.install(Observability::new().with_elasticsearch());
```

### Config

```toml
[observability]
metrics_path = "/metrics"
otel = true
elasticsearch = true
```

### Notes
- Typical order: `request_id` → Observability → `logger`
