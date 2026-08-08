# Sova

Express-like HTTP for Rust: `App`, `Router`, middleware, plugins — Hyper stays hidden.

**Docs:** [https://s00d.github.io/sova/](https://s00d.github.io/sova/) (VitePress)

## Install

```bash
cargo add sova --features web
# or
cargo add sova --features api
```

```rust
use sova::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    App::web()
        .site("Blog")
        .public_url("https://example.com")
        .listen(3000)
        .await
}
```

## Publish (crates.io)

Workspace libs are publishable (`version` + `path` in `[workspace.dependencies]`). Order: **`sova-core` → leaf plugins → dependent plugins → `sova`**. `cargo-sovax` / `sova-docs-gen` stay `publish = false`.

```bash
cargo publish -p sova-core
# …plugins…
cargo publish -p sova
```

## Local docs

```bash
cargo run -p sova-docs-gen   # refresh plugins catalog + Plugin SDK markdown
pnpm docs:dev                # VitePress (reads committed static files)
```

## Layout

- `crates/sova` — facade (`prelude`, plugins behind features)
- `crates/sova-core` — `App`, router, request/response, server
- `crates/cargo-sovax` — install as `cargo-sovax`, run **`cargo sovax …`**
- `plugins/sovax` — in-app `ServerArgs` (feature `cli`)
- `plugins/*` — optional crates
- `docs/` — VitePress site
- `examples/*` — demos (`cargo run -p hello`)

## Coverage

```bash
./scripts/coverage.sh   # ≥80% lines on libraries
```

## Stability

Pre-1.0: breaking changes without a major bump. `sova` **0.1** tracks `sova-core` **0.1**.
