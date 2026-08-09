[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)
[![Donate](https://img.shields.io/badge/Donate-Donationalerts-ff4081?style=for-the-badge)](https://www.donationalerts.com/r/s00d88)

# Public API baselines

Text dumps from [`cargo-public-api`](https://github.com/cargo-public-api/cargo-public-api) (`--simplified`).

| File | Package | Features |
|------|---------|----------|
| `sova-core.txt` | `sova-core` | default (+ `tls` when TLS API changes) |
| `sova.txt` | `sova` facade re-exports | default |
| `sova_store.txt` | `sova_store` | `unstable-store` (compat flag; trait is stable) |
| `sova-tasks-store.txt` | `sova-tasks-store` | `unstable-store` (compat flag; trait is stable) |

Regenerate:

```bash
cargo +nightly public-api -p sova-core --simplified > api/sova-core.txt
cargo +nightly public-api -p sova --simplified > api/sova.txt
cargo +nightly public-api -p sova_store --features unstable-store --simplified > api/sova_store.txt
cargo +nightly public-api -p sova-tasks-store --features unstable-store --simplified > api/sova-tasks-store.txt
```

Check all:

```bash
./scripts/check-public-api.sh
```

When adding TLS or store-crypto, regenerate `sova-core.txt` / `sova.txt` with the matching feature flags if new symbols are exported from the facade.
