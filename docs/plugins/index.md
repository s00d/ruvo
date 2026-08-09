# Plugins

![Plugins](/banners/plugins.svg)

Enable with Cargo features on `sova`. Presets:

```bash
cargo add sova --features web   # HTML apps
cargo add sova --features api   # JSON APIs
cargo add sova --features cors,session,db
```

Open a plugin page from the table. Extra notes for heavier stacks are below.

## Catalog

<!-- generated:plugins-table -->
| Plugin | Version | Summary | Features |
|--------|---------|---------|----------|
| [`activity`](/plugins/activity) | `0.1.1` | Audit / activity log (who changed what) | `activity` |
| [`auth`](/plugins/auth) | `0.1.3` | Register/login, verify, reset, 2FA, profile, roles | `auth`, `auth-mail`, `auth-activity`, `auth-vld` |
| [`compress`](/plugins/compress) | `0.1.0` | gzip / deflate / brotli response compression | `compress` |
| [`cookies`](/plugins/cookies) | `0.1.0` | Parse Cookie header into request-local Cookies | `cookies` |
| [`cors`](/plugins/cors) | `0.1.0` | Cross-Origin Resource Sharing headers | `cors` |
| [`csrf`](/plugins/csrf) | `0.1.0` | Session double-submit CSRF (Laravel-style except/XSRF cookie) | `csrf` |
| [`db`](/plugins/db) | `0.1.2` | SeaORM pool, migrate CLI, optional seed CLI | `db`, `db-mysql`, `db-sqlite` |
| [`env`](/plugins/env) | `0.1.0` | Cascade .env loading for Sova apps (dotenvy) | `env` |
| [`http`](/plugins/http) | `0.1.0` | Outbound HTTP client with SSRF guards and named configs | `http-client` |
| [`i18n`](/plugins/i18n) | `0.1.0` | Locales, catalogs, optional path prefix and cookie | `i18n`, `i18n-cookie` |
| [`mail`](/plugins/mail) | `0.1.0` | Outbound email via lettre (SMTP / fake / file) | `mail`, `mail-markdown`, `mail-templates` |
| [`meta`](/plugins/meta) | `0.1.1` | Document meta, OG/Twitter, JSON-LD, and head inject | `meta`, `meta-openapi`, `meta-i18n`, `meta-store`, `meta-templates` |
| [`notifications`](/plugins/notifications) | `0.1.1` | DB inbox, channels with ACL, optional WS/mail | `notifications`, `notifications-auth`, `notifications-mail`, `notifications-templates`, `notifications-ws` |
| [`observability`](/plugins/observability) | `0.1.0` | HTTP metrics, OpenTelemetry, Elasticsearch log shipping | `observability`, `observability-elasticsearch`, `observability-otel` |
| [`openapi`](/plugins/openapi) | `0.1.0` | OpenAPI 3.1 document + Scalar UI at mount path | `openapi` |
| [`passport`](/plugins/passport) | `0.1.1` | Users + access/refresh JWT + personal access tokens | `passport`, `passport-jwt`, `passport-oauth`, `passport-session` |
| [`quic`](/plugins/quic) | `0.1.0` | QUIC datagrams BackgroundService helpers for Sova | `quic-udp` |
| [`rate-limit`](/plugins/rate-limit) | `0.1.0` | Per-key request rate limiting | `rate-limit` |
| [`redis`](/plugins/redis) | `0.1.0` | Shared Redis/Valkey connection for KvStore, tasks, cache, pub/sub, queues | `redis` |
| [`session`](/plugins/session) | `0.1.0` | Cookie sessions backed by a SessionStore | `session`, `session-redis`, `session-sql` |
| [`shield`](/plugins/shield) | `0.1.0` | Baseline security response headers (helmet-style) | `shield` |
| [`sse`](/plugins/sse) | `0.1.0` | Server-Sent Events helpers for Sova (channels, Last-Event-ID, keep-alive) | `sse-feed` |
| [`static`](/plugins/static) | `0.1.0` | Serve files from a directory under a mount path | `static-files` |
| [`storage`](/plugins/storage) | `0.1.0` | Object storage (local / memory / S3 / GCS / Azure) | `storage`, `storage-azure`, `storage-gcs`, `storage-memory`, `storage-s3` |
| [`store`](/plugins/store) | `0.1.0` | KvStore trait + memory / file / sql / redis backends for Sova | `store`, `store-crypto`, `store-file`, `store-redis`, `store-sql` |
| [`tasks`](/plugins/tasks) | `0.1.0` | Job worker, priorities, and optional cron/interval scheduler | `tasks` |
| [`tasks-store`](/plugins/tasks-store) | `0.1.0` | TaskStore trait + memory / file / sql / redis backends | `tasks-file`, `tasks-redis`, `tasks-sql`, `tasks-store` |
| [`templates`](/plugins/templates) | `0.1.0` | MiniJinja HTML templates with optional autoreload | `templates` |
| [`udp`](/plugins/udp) | `0.1.0` | UDP BackgroundService helpers for Sova | `udp` |
| [`vld`](/plugins/vld) | `0.1.0` | Request validation hooks and coverage check | `vld`, `vld-flash`, `vld-flash-templates`, `vld-form`, `vld-i18n`, `vld-openapi` |
| [`ws`](/plugins/ws) | `0.1.0` | WebSocket hub, origin allowlist, max message size | `ws` |
| [`cli`](/plugins/cli) | `0.1.0` | CLI ServerArgs / listen_args for Sova (local dev) | — |
<!-- /generated:plugins-table -->

## Notes

### Auth (Fortify)

Needs **db + session**. Add **mail** only for `EmailVerification` / `ResetPasswords` (facade feature `auth-mail`).

```rust
app.install(Db::from_env().migrations::<AuthMigrator>());
app.install(memory_sessions()); // or SessionLayer::from_store(...)
// app.install(Mail::from_env()); // only if verify/reset
app.install(
  Fortify::new()
    .features([AuthFeature::Registration, AuthFeature::ResetPasswords])
    .home("/cabinet"),
);
cabinet.use_middleware(Fortify::guard());
```

`auth-activity` writes Fortify mutations to the activity log. Full reference: [auth](/plugins/auth).

### Passport (JWT / PAT / OAuth)

| Kind | Storage | Use |
|------|---------|-----|
| JWT access | signed, short TTL | browsers |
| Refresh | `auth_refresh_tokens` | rotate access |
| PAT (`svpat_…`) | `auth_api_tokens` | machine / CI |

`JwtAuth::guard` accepts Bearer JWT or PAT. OAuth drivers: GitHub / Google / Apple / Custom (`{NAME}_CLIENT_ID` / `_CLIENT_SECRET`). See [passport](/plugins/passport).

### Tasks

Same handlers for Dispatch and CLI. After install: `tasks list` | `schedule` | `run NAME`.

Console (`info` / `ask` / `table`) only during `tasks run`.

```toml
[schedule.ping]
every = "15s"
```

Toml overrides code `.cron()` / `.every()`. See `examples/misc/tasks` and [tasks](/plugins/tasks).

### Database

URL: `DATABASE_URL` or `[db] url`. CLI: `migrate` / `seed` (`cargo sovax db …`). SQL KV/queue backends reuse the same `DbPool`. See [db](/plugins/db).
