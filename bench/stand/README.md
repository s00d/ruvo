# Bench stand — identical multi-page site across frameworks

Shared HTML/JSON fixtures; three **release** servers (Sova, Axum, Actix-web).
Bodies are verified SHA-256 equal before load testing (GET pages + `POST /api/echo`).

```bash
./bench/stand/run.sh                  # deep: warm-up + 3 rounds median
PROFILE=quick ./bench/stand/run.sh    # smoke
./bench/stand/run.sh --update-baseline
ROUNDS=5 DURATION=20s ./bench/stand/run.sh
```

Requires [`oha`](https://github.com/hatoo/oha): `cargo install oha`.

**Stability:** old “91–117k on GET /” was cold-start bias (sova always ran first). Script now does full-concurrency oha warm-up, rotates framework order each round, and reports the **median** (min/max/spread in JSON).

Results: `results/latest.json`, docs page `docs/guide/performance.md`.

Microbenches (also release/`[profile.bench]`):

```bash
cargo bench -p sova-core --bench dispatch
cargo bench -p sova-core --bench dispatch -- --quick
```
