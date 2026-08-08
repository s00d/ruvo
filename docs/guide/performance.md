# Performance

![Performance](/banners/performance.svg)

Sova vs Axum vs Actix-web on an **identical multi-page fixture site** (same HTML/JSON bodies, verified SHA-256).

## Methodology

- Stand: `bench/stand/` — shared fixtures in `fixtures/`, three minimal servers (`stand_sova`, `stand_axum`, `stand_actix`).
- Bodies must match **byte-for-byte** across frameworks before load runs (`run.sh` aborts on mismatch).
- Load tool: [oha](https://github.com/hatoo/oha).
- This capture: duration `10s`, concurrency `50`, `TOKIO_WORKER_THREADS=4`.
- Captured at `2026-08-08T09:42:22Z` on `Admins-MacBook-Pro.local`.

Pages: `/`, `/about`, `/blog`, `/blog/hello`, `/contact`, `/api/health`.

## Latest results — `GET /`

| Framework | Req/s | p50 (ms) | p99 (ms) |
|-----------|-------|----------|----------|
| sova | 101685 | 0.415 | 1.725 |
| axum | 120476 | 0.403 | 0.820 |
| actix | 85351 | 0.509 | 2.347 |

## Latest results — mean across all paths

| Framework | Mean Req/s | Mean p99 (ms) |
|-----------|------------|---------------|
| sova | 103094 | 1.705 |
| axum | 117216 | 1.006 |
| actix | 92173 | 1.899 |

## Per-path detail

| Framework | Path | Req/s | p50 (ms) | p99 (ms) |
|-----------|------|-------|----------|----------|
| sova | `/` | 101685 | 0.415 | 1.725 |
| axum | `/` | 120476 | 0.403 | 0.820 |
| actix | `/` | 85351 | 0.509 | 2.347 |
| sova | `/about` | 112790 | 0.411 | 1.181 |
| axum | `/about` | 118332 | 0.405 | 0.880 |
| actix | `/about` | 94307 | 0.510 | 1.659 |
| sova | `/blog` | 100645 | 0.414 | 1.671 |
| axum | `/blog` | 116968 | 0.404 | 1.046 |
| actix | `/blog` | 91560 | 0.504 | 2.260 |
| sova | `/blog/hello` | 103288 | 0.412 | 1.624 |
| axum | `/blog/hello` | 117281 | 0.408 | 0.944 |
| actix | `/blog/hello` | 94716 | 0.512 | 1.631 |
| sova | `/contact` | 96406 | 0.418 | 2.243 |
| axum | `/contact` | 113758 | 0.407 | 1.146 |
| actix | `/contact` | 93758 | 0.518 | 1.593 |
| sova | `/api/health` | 103748 | 0.408 | 1.784 |
| axum | `/api/health` | 116484 | 0.398 | 1.198 |
| actix | `/api/health` | 93347 | 0.507 | 1.902 |

## Re-run / regression gate

```bash
./bench/stand/run.sh
./bench/stand/run.sh --update-baseline   # after intentional perf changes
DURATION=15s CONCURRENCY=100 ./bench/stand/run.sh
```

Regression thresholds (vs `bench/stand/results/baseline.json`): home RPS drop > 15% or p99 rise > 40% fails the script.

Machine-sensitive: compare relative rankings and deltas, not absolute RPS across laptops.

