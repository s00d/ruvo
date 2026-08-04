# Performance baseline

Captured **before** `pub mod extend` / test split (2026-08-04).

## Criterion (`cargo bench -p ruvo-core --bench dispatch -- --quick`)

| Bench | Median |
|-------|--------|
| `app_build` | 2.31 µs |
| `handle_root` | 683 ns |
| `handle_param` | 810 ns |
| `handle_via_app` | 2.50 µs |

## oha (`bench/load.sh`)

Target: `hello` example on `http://127.0.0.1:3000/`  
`DURATION=10s` `CONCURRENCY=50`

| Metric | Value |
|--------|-------|
| Requests/sec | ~58829 |
| p50 latency | 0.78 ms |
| p99 latency | 2.22 ms |
| Status | 588508 × 200 |

```bash
# Re-run
cargo run -p ruvo --example hello --features "static-files,cors,cookies" &
DURATION=10s CONCURRENCY=50 ./bench/load.sh http://127.0.0.1:3000/
cargo bench -p ruvo-core --bench dispatch -- --quick
```
