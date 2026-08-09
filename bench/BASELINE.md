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

Capture in `bench/stand/results/latest.json` (median of 3 rounds, oha warm-up, rotating fw order):

| Framework | GET / (median) | POST /api/echo | Mean Req/s |
|-----------|----------------|----------------|------------|
| sova | ~122k | ~123k | ~118k |
| axum | ~123k | ~126k | ~120k |
| actix | ~101k | ~103k | ~104k |

Previous “91–117k on GET /” was cold-start bias (sova always first). Max round spread is reported per path in JSON (`rps_spread_pct`).

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
