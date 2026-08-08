# Bench stand — identical multi-page site across frameworks

Shared HTML/JSON fixtures; three minimal servers (Ruvo, Axum, Actix-web).
Bodies are verified SHA-256 equal before load testing.

```bash
./bench/stand/run.sh
./bench/stand/run.sh --update-baseline
DURATION=15s CONCURRENCY=100 ./bench/stand/run.sh
```

Requires [`oha`](https://github.com/hatoo/oha): `cargo install oha`.

Results: `results/latest.json`, docs page `docs/guide/performance.md`.
