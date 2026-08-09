[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)
[![Donate](https://img.shields.io/badge/Donate-Donationalerts-ff4081?style=for-the-badge)](https://www.donationalerts.com/r/s00d88)

# plugins/

Opt-in domain plugins for Sova. Prefer enabling them through facade features on [`sova`](https://crates.io/crates/sova).

| Crate | Version | Description | Docs |
|-------|---------|-------------|------|
| [`sova-activity`](./sova-activity/) | `0.1.2` | Laravel-style activity / audit log for Sova | [guide](https://s00d.github.io/sova/plugins/activity) |
| [`sova-ai`](./sova-ai/) | `0.1.0` | AISDK language models for Sova (chat, tools, stream, fake) | [guide](https://s00d.github.io/sova/plugins/ai) |
| [`sova-auth`](./sova-auth/) | `0.1.6` | Fortify-style authentication for Sova (register, 2FA, reset, roles) | [guide](https://s00d.github.io/sova/plugins/auth) |
| [`sova-compress`](./sova-compress/) | `0.1.1` | Response compression plugin for Sova | [guide](https://s00d.github.io/sova/plugins/compress) |
| [`sova-cookies`](./sova-cookies/) | `0.1.1` | Cookie parsing plugin for Sova | [guide](https://s00d.github.io/sova/plugins/cookies) |
| [`sova-cors`](./sova-cors/) | `0.1.1` | CORS plugin for Sova | [guide](https://s00d.github.io/sova/plugins/cors) |
| [`sova-csrf`](./sova-csrf/) | `0.1.2` | CSRF protection plugin for Sova (session double-submit) | [guide](https://s00d.github.io/sova/plugins/csrf) |
| [`sova-db`](./sova-db/) | `0.1.3` | SeaORM database plugin for Sova (postgres / sqlite / mysql) | [guide](https://s00d.github.io/sova/plugins/db) |
| [`sova-devtools`](./sova-devtools/) | `0.1.3` | In-app DevTools bar (HTML inject, SSE timeline) | [guide](https://s00d.github.io/sova/plugins/devtools) |
| [`sova-env`](./sova-env/) | `0.1.1` | Cascade .env loading for Sova apps (dotenvy) | [guide](https://s00d.github.io/sova/plugins/env) |
| [`sova-http`](./sova-http/) | `0.1.1` | Outbound HTTP client bound to request deadline and tracing | [guide](https://s00d.github.io/sova/plugins/http) |
| [`sova-i18n`](./sova-i18n/) | `0.1.2` | Two-level i18n store and locale resolution for Sova | [guide](https://s00d.github.io/sova/plugins/i18n) |
| [`sova-mail`](./sova-mail/) | `0.1.1` | Outbound email for Sova (lettre SMTP / fake / file) | [guide](https://s00d.github.io/sova/plugins/mail) |
| [`sova-meta`](./sova-meta/) | `0.1.2` | Document meta, OG/Twitter, JSON-LD, sitemap and robots for Sova | [guide](https://s00d.github.io/sova/plugins/meta) |
| [`sova-notifications`](./sova-notifications/) | `0.1.4` | Database notifications with channels, ACL, optional WS/mail | [guide](https://s00d.github.io/sova/plugins/notifications) |
| [`sova-observability`](./sova-observability/) | `0.1.1` | Request metrics (Prometheus), OpenTelemetry, and Elasticsearch logs for Sova | [guide](https://s00d.github.io/sova/plugins/observability) |
| [`sova-openapi`](./sova-openapi/) | `0.1.1` | OpenAPI document generation and /docs UI for Sova | [guide](https://s00d.github.io/sova/plugins/openapi) |
| [`sova-passport`](./sova-passport/) | `0.1.2` | Passport-style authentication for Sova (strategies, JWT, OAuth2, sessions) | [guide](https://s00d.github.io/sova/plugins/passport) |
| [`sova-quic`](./sova-quic/) | `0.1.1` | QUIC datagrams BackgroundService helpers for Sova | [guide](https://s00d.github.io/sova/plugins/quic) |
| [`sova-rate-limit`](./sova-rate-limit/) | `0.1.1` | Rate limit plugin for Sova (local sliding + shared fixed window) | [guide](https://s00d.github.io/sova/plugins/rate-limit) |
| [`sova-redis`](./sova-redis/) | `0.1.2` | Shared Redis/Valkey pool plugin for Sova | [guide](https://s00d.github.io/sova/plugins/redis) |
| [`sova-session`](./sova-session/) | `0.1.2` | Session plugin for Sova | [guide](https://s00d.github.io/sova/plugins/session) |
| [`sova-shield`](./sova-shield/) | `0.1.1` | Security headers middleware for Sova | [guide](https://s00d.github.io/sova/plugins/shield) |
| [`sova-sse`](./sova-sse/) | `0.1.1` | Server-Sent Events helpers for Sova (channels, Last-Event-ID, keep-alive) | [guide](https://s00d.github.io/sova/plugins/sse) |
| [`sova-static`](./sova-static/) | `0.1.1` | Static file plugin for Sova | [guide](https://s00d.github.io/sova/plugins/static) |
| [`sova-storage`](./sova-storage/) | `0.1.1` | Object storage for Sova (local / memory / S3 / GCS / Azure) | [guide](https://s00d.github.io/sova/plugins/storage) |
| [`sova-store`](./sova-store/) | `0.1.2` | KvStore trait + memory / file / sql / redis backends for Sova | [guide](https://s00d.github.io/sova/plugins/store) |
| [`sova-tasks`](./sova-tasks/) | `0.1.2` | Task worker / scheduler / HTTP enqueue for Sova | [guide](https://s00d.github.io/sova/plugins/tasks) |
| [`sova-tasks-store`](./sova-tasks-store/) | `0.1.1` | TaskStore trait + memory / file / sql / redis backends | [guide](https://s00d.github.io/sova/plugins/tasks-store) |
| [`sova-templates`](./sova-templates/) | `0.1.1` | Template engine plugin for Sova | [guide](https://s00d.github.io/sova/plugins/templates) |
| [`sova-udp`](./sova-udp/) | `0.1.1` | UDP BackgroundService helpers for Sova | [guide](https://s00d.github.io/sova/plugins/udp) |
| [`sova-vld`](./sova-vld/) | `0.1.3` | vld validation integration for Sova | [guide](https://s00d.github.io/sova/plugins/vld) |
| [`sova-ws`](./sova-ws/) | `0.1.1` | WebSocket plugin for Sova | [guide](https://s00d.github.io/sova/plugins/ws) |
| [`sovax`](./sovax/) | `0.1.1` | CLI ServerArgs / listen_args for Sova (local dev) | [guide](https://s00d.github.io/sova/plugins/cli) |

Each plugin ships `README.md` + `LICENSE` (MIT). Catalog: [https://s00d.github.io/sova/plugins/](https://s00d.github.io/sova/plugins/).
