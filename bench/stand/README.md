# Bench stand — identical multi-page site across frameworks

Shared HTML/JSON fixtures; three **release** servers (Sova, Axum, Actix-web).
Bodies are verified SHA-256 equal before load testing (GET pages + `POST /api/echo`).

```bash
./bench/stand/run.sh                  # deep defaults: 30s, c=100, release+LTO
PROFILE=quick ./bench/stand/run.sh    # smoke: 10s, c=50
./bench/stand/run.sh --update-baseline
DURATION=60s CONCURRENCY=200 ./bench/stand/run.sh
```

Requires [`oha`](https://github.com/hatoo/oha): `cargo install oha`.

Results: `results/latest.json`, docs page `docs/guide/performance.md`.

Microbenches (also release/`[profile.bench]`):

```bash
cargo bench -p sova-core --bench dispatch
cargo bench -p sova-core --bench dispatch -- --quick
```
