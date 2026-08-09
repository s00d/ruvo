# Performance baseline

Updated **2026-08-09** after hot-path work (`FxHashMap` / rustc-hash, Arc `MetaMap`,
lazy catcher wrap, skip raw lookup, no response header clone, HEAD `Content-Length`,
request-id entropy) and deeper **release** benches.

## Criterion (`cargo bench -p sova-core --bench dispatch -- --quick`)

Profile: workspace `[profile.bench]` → release + thin LTO.

| Bench | Median |
|-------|--------|
| `build/minimal` | ~2.72 µs |
| `build/realistic` | ~8.25 µs |
| `handle_minimal/root` | ~703 ns |
| `handle_minimal/param` | ~846 ns |
| `handle_realistic/home` | ~1.08 µs |
| `handle_realistic/api_user` | ~1.36 µs |
| `handle_realistic/echo_json` | ~1.35 µs |
| `burst/256_mixed` | ~366 µs / 256 req (~1.43 µs/req) |

Misleading `handle_via_app` (recompiled every call) was removed.

## Stand (`./bench/stand/run.sh`)

Release binaries, identical fixtures, SHA-256 verify including `POST /api/echo`.

Capture in `bench/stand/results/latest.json` (example: `DURATION=20s` `CONCURRENCY=100` workers=4):

| Framework | GET / Req/s | POST /api/echo | Mean Req/s |
|-----------|-------------|----------------|------------|
| sova | ~91–117k* | ~117k | ~113k |
| axum | ~113k | ~118k | ~122k |
| actix | ~108k | ~105k | ~104k |

\*First path can be noisy before warm-up; script now warms caches. Prefer mean / per-path table in `docs/guide/performance.md`.

```bash
./bench/stand/run.sh
PROFILE=quick ./bench/stand/run.sh
cargo bench -p sova-core --bench dispatch
```

## SCALE (`bench/scale.sh`)

Older shard numbers for session/rate-limit still in git history; re-run after store changes:

```bash
DURATION=5s ./bench/scale.sh both
```
