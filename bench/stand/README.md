[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)
[![Donate](https://img.shields.io/badge/Donate-Donationalerts-ff4081?style=for-the-badge)](https://www.donationalerts.com/r/s00d88)

# Bench stand — identical multi-page site across frameworks

Shared HTML/JSON fixtures; three minimal servers (Sova, Axum, Actix-web).
Bodies are verified SHA-256 equal before load testing.

```bash
./bench/stand/run.sh
./bench/stand/run.sh --update-baseline
DURATION=15s CONCURRENCY=100 ./bench/stand/run.sh
```

Requires [`oha`](https://github.com/hatoo/oha): `cargo install oha`.

Results: `results/latest.json`, docs page `docs/guide/performance.md`.
