<p align="center">
  <img src="assets/sova-header.svg" alt="Sova" width="720" />
</p>

# Sova

Express-like HTTP for Rust: `App`, `Router`, middleware, plugins — Hyper stays hidden.

**Docs:** [https://s00d.github.io/sova/](https://s00d.github.io/sova/)

## Install

```bash
cargo add sova --features web
# or
cargo add sova --features api
```

## Example

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
