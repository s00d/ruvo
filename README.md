# Ruvo

Express-like HTTP for Rust: `App`, `Router`, middleware, plugins — Hyper stays hidden.

**Docs:** [https://s00d.github.io/ruvo/](https://s00d.github.io/ruvo/) (VitePress)

## Install

```bash
cargo add ruvo --features web
# or
cargo add ruvo --features api
```

```rust
use ruvo::prelude::*;

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

Workspace libs are publishable (`version` + `path` in `[workspace.dependencies]`). Order: **`ruvo-core` → leaf plugins → dependent plugins → `ruvo`**. `cargo-ruvo` / `ruvo-docs-gen` stay `publish = false`.

```bash
cargo publish -p ruvo-core
# …plugins…
cargo publish -p ruvo
```

## Local docs

```bash
cargo run -p ruvo-docs-gen   # refresh plugins catalog + Plugin SDK markdown
pnpm docs:dev                # VitePress (reads committed static files)
```

## Layout

- `crates/ruvo` — facade (`prelude`, plugins behind features)
- `crates/ruvo-core` — `App`, router, request/response, server
- `crates/cargo-ruvo` — `cargo ruvo new` / `generate` / `dev` / `db`
- `plugins/*` — optional crates
- `docs/` — VitePress site
- `examples/*` — demos (`cargo run -p hello`)

## Coverage

```bash
./scripts/coverage.sh   # ≥80% lines on libraries
```

## Stability

Pre-1.0: breaking changes without a major bump. `ruvo` **0.1** tracks `ruvo-core` **0.1**.
