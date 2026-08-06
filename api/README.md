# Public API baselines

Text dumps from [`cargo-public-api`](https://github.com/cargo-public-api/cargo-public-api) (`--simplified`).

| File | Package | Features |
|------|---------|----------|
| `ruvo-core.txt` | `ruvo-core` | default (+ `tls` when TLS API changes) |
| `ruvo.txt` | `ruvo` facade re-exports | default |
| `ruvo-store.txt` | `ruvo-store` | `unstable-store` (compat flag; trait is stable) |
| `ruvo-tasks-store.txt` | `ruvo-tasks-store` | `unstable-store` (compat flag; trait is stable) |

Regenerate:

```bash
cargo +nightly public-api -p ruvo-core --simplified > api/ruvo-core.txt
cargo +nightly public-api -p ruvo --simplified > api/ruvo.txt
cargo +nightly public-api -p ruvo-store --features unstable-store --simplified > api/ruvo-store.txt
cargo +nightly public-api -p ruvo-tasks-store --features unstable-store --simplified > api/ruvo-tasks-store.txt
```

Check all:

```bash
./scripts/check-public-api.sh
```

When adding TLS or store-crypto, regenerate `ruvo-core.txt` / `ruvo.txt` with the matching feature flags if new symbols are exported from the facade.
