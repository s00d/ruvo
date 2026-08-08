[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![downloads](https://img.shields.io/crates/d/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)
[![Donate](https://img.shields.io/badge/Donate-Donationalerts-ff4081?style=for-the-badge)](https://www.donationalerts.com/r/s00d88)

<p align="center">
  <img src="https://raw.githubusercontent.com/s00d/sova/master/assets/branding/sova-readme-banner.png?v=8" alt="Sova" width="720" />
</p>

# Sova

Very simple HTTP framework for Rust, inspired by Express on Node.js.

You get an `App`, a `Router`, middleware, and optional plugins — without drowning in Tower/Hyper boilerplate. Presets like `App::web()` / `App::api()` wire a sensible stack so you can ship a site or JSON API quickly, then turn features on as you need them.

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
    let mut app = App::web()
        .site("Hello")
        .public_url("http://127.0.0.1:3000");

    app.get("/", |_req| async { Ok("hello from sova") });

    app.listen(3000).await
}
```

## Plugins

`activity` · `auth` · `cli` · `compress` · `cookies` · `cors` · `csrf` · `db` · `env` · `http` · `i18n` · `mail` · `meta` · `notifications` · `observability` · `openapi` · `passport` · `quic` · `rate-limit` · `redis` · `session` · `shield` · `sse` · `static` · `storage` · `store` · `tasks` · `tasks-store` · `templates` · `udp` · `vld` · `ws`

Details and guides: [https://s00d.github.io/sova/](https://s00d.github.io/sova/)
