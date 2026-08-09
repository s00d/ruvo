---
title: observability
editLink: false
---

# `observability`

**HTTP metrics, OpenTelemetry, Elasticsearch log shipping**

| | |
|--|--|
| Crate | [`sova-observability`](https://docs.rs/sova-observability/0.1.2) `0.1.2` |
| Plugin id | `observability` |
| Category | Ops |

## Install

```bash
cargo add sova --features observability
```

## Features

| Feature | What you get |
|---------|-------------|
| `observability` | Prometheus `/metrics` + HTTP RED middleware. |
| `observability-elasticsearch` | Ship tracing logs to Elasticsearch. |
| `observability-otel` | OpenTelemetry OTLP export. |

## Overview

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

## Quick start

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

```toml
[observability]
metrics_path = "/metrics"
otel = true
elasticsearch = true
```

Env: `OTEL_EXPORTER_OTLP_ENDPOINT`, `ELASTICSEARCH_URL` (+ user/password/api key).

## Examples

- [`examples/misc/bench_loaded`](https://github.com/s00d/sova/tree/master/examples/misc/bench_loaded)

## Related

[`activity`](/plugins/activity)
