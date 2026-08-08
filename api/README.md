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
