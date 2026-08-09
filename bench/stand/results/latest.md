# Performance

![Performance](/banners/performance.svg)

Sova vs Axum vs Actix-web on an **identical multi-page fixture site** (same HTML/JSON bodies, verified SHA-256), including a realistic **POST /api/echo** JSON path.

## Methodology

- Stand: `bench/stand/` — shared fixtures, three **release** servers (`stand_sova`, `stand_axum`, `stand_actix`).
- Workspace `[profile.release]` uses thin LTO (`codegen-units = 1`) — production-shaped binaries, not `dev`.
- Bodies must match **byte-for-byte** across frameworks before load runs (`run.sh` aborts on mismatch).
- Load tool: [oha](https://github.com/hatoo/oha).
- This capture: profile `deep`, duration `20s`, concurrency `100`, `TOKIO_WORKER_THREADS=4`.
- Captured at `2026-08-09T05:56:00Z` on `Admins-MacBook-Pro.local`.

Pages: `/`, `/about`, `/blog`, `/blog/hello`, `/contact`, `/api/health`, `POST /api/echo`.

## Latest results — `GET /`

| Framework | Req/s | p50 (ms) | p99 (ms) |
|-----------|-------|----------|----------|
| sova | 91300 | 0.912 | 3.636 |
| axum | 112627 | 0.796 | 2.353 |
| actix | 107950 | 0.906 | 3.083 |

## Latest results — `POST /api/echo`

| Framework | Req/s | p99 (ms) |
|-----------|-------|----------|
| sova | 116927 | 2.389 |
| axum | 118473 | 2.563 |
| actix | 105296 | 4.344 |

## Latest results — mean across all paths

| Framework | Mean Req/s | Mean p99 (ms) |
|-----------|------------|---------------|
| sova | 112636 | 2.522 |
| axum | 121632 | 1.997 |
| actix | 104190 | 3.186 |

## Per-path detail

| Framework | Path | Req/s | p50 (ms) | p99 (ms) |
|-----------|------|-------|----------|----------|
| sova | `/` | 91300 | 0.912 | 3.636 |
| axum | `/` | 112627 | 0.796 | 2.353 |
| actix | `/` | 107950 | 0.906 | 3.083 |
| sova | `/about` | 113972 | 0.787 | 2.485 |
| axum | `/about` | 124915 | 0.770 | 1.754 |
| actix | `/about` | 100388 | 0.919 | 3.691 |
| sova | `/blog` | 117259 | 0.785 | 2.139 |
| axum | `/blog` | 122222 | 0.775 | 2.053 |
| actix | `/blog` | 101180 | 0.909 | 3.405 |
| sova | `/blog/hello` | 115320 | 0.786 | 2.430 |
| axum | `/blog/hello` | 122531 | 0.773 | 1.969 |
| actix | `/blog/hello` | 101324 | 0.920 | 3.033 |
| sova | `/contact` | 117228 | 0.780 | 2.292 |
| axum | `/contact` | 125754 | 0.771 | 1.628 |
| actix | `/contact` | 102498 | 0.921 | 2.481 |
| sova | `/api/health` | 116448 | 0.772 | 2.286 |
| axum | `/api/health` | 124904 | 0.762 | 1.658 |
| actix | `/api/health` | 110692 | 0.904 | 2.264 |
| sova | `/api/echo` | 116927 | 0.764 | 2.389 |
| axum | `/api/echo` | 118473 | 0.764 | 2.563 |
| actix | `/api/echo` | 105296 | 0.873 | 4.344 |

## Re-run / regression gate

```bash
./bench/stand/run.sh                  # deep (30s, c=100) release
PROFILE=quick ./bench/stand/run.sh   # smoke
./bench/stand/run.sh --update-baseline
DURATION=60s CONCURRENCY=200 ./bench/stand/run.sh
cargo bench -p sova-core --bench dispatch   # release criterion
```

Regression thresholds (vs `bench/stand/results/baseline.json`): home RPS drop > 15% or p99 rise > 40% fails the script.

Machine-sensitive: compare relative rankings and deltas, not absolute RPS across laptops.

