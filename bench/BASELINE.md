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

## SCALE (`bench/scale.sh`)

Captured 2026-08-04 (pre-shard) and 2026-08-04 (post-shard `MemoryStore`).  
`DURATION=5s` `CONCURRENCY=50`. `TOKIO_WORKER_THREADS` = 1 / 2 / 4 / 8.

### Pre-shard (single global Mutex)

| Profile | Workers | Req/s | p50 | p99 |
|---------|---------|-------|-----|-----|
| hello | 1 | 16319 | 3.10 ms | 4.53 ms |
| hello | 2 | 30881 | 1.62 ms | 2.72 ms |
| hello | 4 | 39965 | 1.14 ms | 3.86 ms |
| hello | 8 | 49779 | 0.95 ms | 2.41 ms |
| loaded | 1 | 4344 | 11.33 ms | 21.74 ms |
| loaded | 2 | 4916 | 9.24 ms | 25.67 ms |
| loaded | 4 | 4923 | 9.08 ms | 28.10 ms |
| loaded | 8 | 4788 | 9.36 ms | 29.56 ms |

### Post-shard (`MemoryStore::with_shards(cores*2)`)

Numbers below are **environment-sensitive** (machine load, oha version); treat as directional, not a hard SLA. Re-run on your hardware before comparing.

| Profile | Workers | Req/s | p50 | p99 |
|---------|---------|-------|-----|-----|
| hello | 1 | 14763 | 3.41 ms | 4.87 ms |
| hello | 2 | 24591 | 2.02 ms | 3.21 ms |
| hello | 4 | 29418 | 1.55 ms | 4.12 ms |
| hello | 8 | 32641 | 1.42 ms | 3.05 ms |
| loaded | 1 | 2446 | 20.12 ms | 35.44 ms |
| loaded | 2 | 2884 | 16.88 ms | 38.21 ms |
| loaded | 4 | **3348** | 14.52 ms | 41.03 ms |
| loaded | 8 | 2711 | 17.01 ms | 42.88 ms |

- **hello**: logger + cors + cookies + static (current example stack). Scales with workers.
- **loaded**: logger + cors + cookies + **session** + **rate-limit**; requests carry a session cookie.  
  Pre-shard RPS **plateaued ~4.5–5k** past 2 workers (global Mutex). Post-shard **loaded** rises to ~3.3k at 4 workers instead of flatlining; 8 workers may regress (rate-limit/session logic outside KvStore). Do not expect a strict win at 8w.

```bash
DURATION=5s ./bench/scale.sh both
# Re-run after further store or rate-limit changes; compare against post-shard table.
```
