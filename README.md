[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![downloads](https://img.shields.io/crates/d/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![Donate](https://img.shields.io/badge/Donate-Donationalerts-ff4081?style=for-the-badge)](https://www.donationalerts.com/r/s00d88)

<p align="center">
  <img src="assets/sova-header.svg?v=2" alt="Sova" width="720" />
</p>

# Sova

Express-like HTTP framework for Rust: `App`, `Router`, middleware, and plugins — Hyper stays under the hood.

**Documentation:** [https://s00d.github.io/sova/](https://s00d.github.io/sova/)

## Install

```bash
cargo add sova --features web   # HTML apps
# or
cargo add sova --features api   # JSON APIs
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

## Plugins

Enable with Cargo features on `sova`. Full catalog and usage: [Plugins](https://s00d.github.io/sova/plugins/).

| Plugin | Summary |
|--------|---------|
| [activity](https://s00d.github.io/sova/plugins/activity) | Audit / activity log |
| [auth](https://s00d.github.io/sova/plugins/auth) | Register/login, verify, reset, 2FA, roles |
| [cli](https://s00d.github.io/sova/plugins/cli) | `ServerArgs` / local CLI helpers (`sovax`) |
| [compress](https://s00d.github.io/sova/plugins/compress) | gzip / deflate / brotli |
| [cookies](https://s00d.github.io/sova/plugins/cookies) | Cookie jar |
| [cors](https://s00d.github.io/sova/plugins/cors) | CORS |
| [csrf](https://s00d.github.io/sova/plugins/csrf) | Session CSRF / XSRF |
| [db](https://s00d.github.io/sova/plugins/db) | SeaORM pool, migrate, seed |
| [env](https://s00d.github.io/sova/plugins/env) | Cascade `.env` loading |
| [http](https://s00d.github.io/sova/plugins/http) | Outbound HTTP + SSRF guards |
| [i18n](https://s00d.github.io/sova/plugins/i18n) | Locales and catalogs |
| [mail](https://s00d.github.io/sova/plugins/mail) | SMTP / fake / file mailer |
| [meta](https://s00d.github.io/sova/plugins/meta) | SEO meta, OG, sitemap, robots |
| [notifications](https://s00d.github.io/sova/plugins/notifications) | DB inbox, WS/mail channels |
| [observability](https://s00d.github.io/sova/plugins/observability) | Metrics, OTel, Elasticsearch |
| [openapi](https://s00d.github.io/sova/plugins/openapi) | OpenAPI 3.1 + Scalar UI |
| [passport](https://s00d.github.io/sova/plugins/passport) | JWT / PAT / OAuth |
| [quic](https://s00d.github.io/sova/plugins/quic) | QUIC datagrams |
| [rate-limit](https://s00d.github.io/sova/plugins/rate-limit) | Per-key rate limiting |
| [redis](https://s00d.github.io/sova/plugins/redis) | Redis / Valkey pool |
| [session](https://s00d.github.io/sova/plugins/session) | Cookie sessions |
| [shield](https://s00d.github.io/sova/plugins/shield) | Security headers |
| [sse](https://s00d.github.io/sova/plugins/sse) | Server-Sent Events |
| [static](https://s00d.github.io/sova/plugins/static) | Static files |
| [storage](https://s00d.github.io/sova/plugins/storage) | Local / S3 / GCS / Azure |
| [store](https://s00d.github.io/sova/plugins/store) | KvStore backends |
| [tasks](https://s00d.github.io/sova/plugins/tasks) | Jobs, worker, scheduler |
| [tasks-store](https://s00d.github.io/sova/plugins/tasks-store) | Task queue backends |
| [templates](https://s00d.github.io/sova/plugins/templates) | MiniJinja HTML |
| [udp](https://s00d.github.io/sova/plugins/udp) | UDP services |
| [vld](https://s00d.github.io/sova/plugins/vld) | Request validation |
| [ws](https://s00d.github.io/sova/plugins/ws) | WebSockets |

Tooling: install `cargo-sovax`, run **`cargo sovax …`** for scaffold / `dev` / `db`.
