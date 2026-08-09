[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![downloads](https://img.shields.io/crates/d/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)
[![Donate](https://img.shields.io/badge/Donate-Donationalerts-ff4081?style=for-the-badge)](https://www.donationalerts.com/r/s00d88)

<p align="center">
  <img src="https://raw.githubusercontent.com/s00d/sova/master/assets/branding/sova-readme-banner.png" alt="Sova" width="720" />
</p>

# Sova

Sova exists to make writing HTTP servers in Rust feel fast again.

The goal is not a line-for-line port of Express. Express won because it stayed out of the way: an app, routes, middleware, and enough structure to grow without ceremony. Sova aims at that same lightness — familiar shapes (`App`, `Router`, middleware, plugins) on top of Rust’s performance and type system, without dragging you through Tower/Hyper boilerplate just to say hello.

**Core stays thin.** It is a small wrapper around the HTTP foundation: request path, routing, the middleware onion, and a plugin hook. It does not try to own every concern of a production app.

**Plugins own their domains.** Session, auth, DB, mail, storage, WebSockets, and the rest are opt-in crates. Each one fully owns the logic it is responsible for; you install what you need and leave the rest out. Presets like `App::web()` / `App::api()` wire a sensible starting stack so you can ship a site or JSON API quickly, then grow feature by feature.

There is already a solid pack of ready plugins for common server work — see the list below and the docs for guides.

**Docs:** [https://s00d.github.io/sova/](https://s00d.github.io/sova/)  
**Changelog:** [CHANGELOG.md](CHANGELOG.md)  
**License:** [MIT](LICENSE)

## Install

```bash
cargo add sova --features web
# or
cargo add sova --features api
# or both (then pick App::web() or App::api() and install the rest)
cargo add sova --features "web,api"
cargo add tokio --features "rt-multi-thread,macros"
```

Scaffold: `cargo install cargo-sovax`, then `cargo sovax new myapp --web` / `--api` (templates already include `tokio`).

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
