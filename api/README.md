# Public API baselines

Text dumps from [`cargo-public-api`](https://github.com/cargo-public-api/cargo-public-api) (`--simplified`).

| File | Package |
|------|---------|
| `ruvo-core.txt` | `ruvo-core` (handlers, Request/Response, App/Router) |

The `ruvo` facade is mostly `pub use ruvo_core::*`; baseline the core crate.

Regenerate:

```bash
cargo +nightly public-api -p ruvo-core --simplified > api/ruvo-core.txt
```

Check:

```bash
./scripts/check-public-api.sh
```
