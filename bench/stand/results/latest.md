# Performance

![Performance](/banners/performance.svg)

Sova vs Axum vs Actix-web on an **identical multi-page fixture site** (same HTML/JSON bodies, verified SHA-256), including a realistic **POST /api/echo** JSON path.

## Methodology

- Stand: `bench/stand/` — shared fixtures, three **release** servers (`stand_sova`, `stand_axum`, `stand_actix`).
- Workspace `[profile.release]` uses thin LTO (`codegen-units = 1`) — production-shaped binaries, not `dev`.
- Bodies must match **byte-for-byte** across frameworks before load runs.
- Load tool: [oha](https://github.com/hatoo/oha).
- Stability: oha warm-up `5s` at full concurrency; **3 round(s)** with rotating framework order; reported numbers are the **median** RPS (min/max kept in JSON for spread).
- This capture: profile `deep`, duration `15s`, concurrency `100`, `TOKIO_WORKER_THREADS=4`.
- Captured at `2026-08-09T06:19:10Z` on `Admins-MacBook-Pro.local`.

Pages: `/`, `/about`, `/blog`, `/blog/hello`, `/contact`, `/api/health`, `POST /api/echo`.

## Latest results — `GET /` (median)

| Framework | Req/s | min–max | p50 (ms) | p99 (ms) |
|-----------|-------|---------|----------|----------|
| sova | 122246 | 108062–123173 | 0.785 | 1.831 |
| axum | 122665 | 115021–124864 | 0.783 | 1.782 |
| actix | 100649 | 99291–104136 | 0.934 | 2.712 |

## Latest results — `POST /api/echo` (median)

| Framework | Req/s | p99 (ms) |
|-----------|-------|----------|
| sova | 122813 | 2.106 |
| axum | 125580 | 1.947 |
| actix | 102865 | 3.013 |

## Latest results — mean across all paths (of medians)

| Framework | Mean Req/s | Mean p99 (ms) | max path spread % |
|-----------|------------|---------------|-------------------|
| sova | 117589 | 2.272 | 12.4% |
| axum | 120448 | 2.083 | 8.6% |
| actix | 103513 | 3.017 | 7.4% |

## Per-path detail (median)

| Framework | Path | Req Req/s | min–max | spread % | p50 | p99 |
|-----------|------|------------|---------|----------|-----|-----|
| actix | `/` | 100649 | 99291–104136 | 4.8% | 0.934 | 2.712 |
| actix | `/about` | 104676 | 99866–105918 | 5.8% | 0.929 | 2.751 |
| actix | `/api/echo` | 102865 | 102147–106977 | 4.7% | 0.902 | 3.013 |
| actix | `/api/health` | 107683 | 100091–108013 | 7.4% | 0.903 | 2.783 |
| actix | `/blog` | 99644 | 98465–103154 | 4.7% | 0.925 | 3.448 |
| actix | `/blog/hello` | 104482 | 103196–106699 | 3.4% | 0.908 | 3.900 |
| actix | `/contact` | 104591 | 100530–106134 | 5.4% | 0.921 | 2.511 |
| axum | `/` | 122665 | 115021–124864 | 8.0% | 0.783 | 1.782 |
| axum | `/about` | 123469 | 115999–125375 | 7.6% | 0.781 | 1.740 |
| axum | `/api/echo` | 125580 | 117318–126586 | 7.4% | 0.762 | 1.947 |
| axum | `/api/health` | 125295 | 123798–126208 | 1.9% | 0.766 | 1.774 |
| axum | `/blog` | 115613 | 114235–122167 | 6.9% | 0.779 | 2.220 |
| axum | `/blog/hello` | 117184 | 112162–122204 | 8.6% | 0.782 | 2.384 |
| axum | `/contact` | 113332 | 113126–120328 | 6.4% | 0.779 | 2.737 |
| sova | `/` | 122246 | 108062–123173 | 12.4% | 0.785 | 1.831 |
| sova | `/about` | 115248 | 112862–115342 | 2.2% | 0.786 | 2.502 |
| sova | `/api/echo` | 122813 | 120465–124537 | 3.3% | 0.763 | 2.106 |
| sova | `/api/health` | 114522 | 113884–115791 | 1.7% | 0.773 | 2.462 |
| sova | `/blog` | 118889 | 110540–119304 | 7.4% | 0.786 | 2.180 |
| sova | `/blog/hello` | 110984 | 110878–114174 | 3.0% | 0.792 | 2.712 |
| sova | `/contact` | 118417 | 109367–121292 | 10.1% | 0.787 | 2.111 |

## Re-run / regression gate

```bash
./bench/stand/run.sh                     # deep: 15s × 3 rounds, warm-up, median
PROFILE=quick ./bench/stand/run.sh      # smoke
./bench/stand/run.sh --update-baseline
ROUNDS=5 DURATION=20s ./bench/stand/run.sh
```

Regression thresholds (vs `bench/stand/results/baseline.json`): home RPS drop > 15% or p99 rise > 40% fails the script.

Machine-sensitive: compare relative rankings and deltas, not absolute RPS across laptops.

