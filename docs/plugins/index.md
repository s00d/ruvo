# Plugins

![Plugins](/banners/plugins.svg)

Opt-in crates behind `sova` Cargo features. Start from a preset, then install only what you need.

```bash
cargo add sova --features web   # HTML apps (cors, shield, session, csrf, static, templates, meta, …)
cargo add sova --features api   # JSON APIs (cors, session, openapi, …)
cargo add sova --features db,auth,mail
```

Each plugin page: **what / when**, features, overview, quick start, related links, example apps.

How pages are built: crate `//!` + optional [`plugin-guides`](https://github.com/s00d/sova/tree/master/docs/.vitepress/plugin-guides) / [`plugin-usage`](https://github.com/s00d/sova/tree/master/docs/.vitepress/plugin-usage) → `sova-docs-gen`.

Writing a new plugin? See the [Plugin SDK](/api/plugin-sdk) guide (same generator, guides under `plugin-sdk-guides/`).

## Catalog

<!-- generated:plugins-table -->
| Plugin | Category | Version | Summary | Features |
|--------|----------|---------|---------|----------|
| [`acme`](/plugins/acme) | HTTP | `0.1.1` | Let's Encrypt HTTP-01 certificates with TLS hot-reload | `acme` |
| [`activity`](/plugins/activity) | Ops | `0.1.2` | Audit / activity log (who changed what) | `activity` |
| [`ai`](/plugins/ai) | Integrations | `0.1.0` | AISDK language models (chat, tools, stream, fake) | `ai`, `ai-anthropic`, `ai-full`, `ai-google`, `ai-openai`, `ai-prompt` |
| [`auth`](/plugins/auth) | Auth | `0.1.8` | Register/login, verify, reset, 2FA, profile, roles | `auth`, `auth-activity`, `auth-mail`, `auth-vld` |
| [`compress`](/plugins/compress) | HTTP | `0.1.1` | gzip / deflate / brotli response compression | `compress` |
| [`cookies`](/plugins/cookies) | HTTP | `0.1.1` | Parse Cookie header into request-local Cookies | `cookies` |
| [`cors`](/plugins/cors) | HTTP | `0.1.1` | Cross-Origin Resource Sharing headers | `cors` |
| [`csrf`](/plugins/csrf) | HTTP | `0.1.3` | Session double-submit CSRF (Laravel-style except/XSRF cookie) | `csrf` |
| [`db`](/plugins/db) | Data | `0.1.3` | SeaORM pool, migrate CLI, optional seed CLI | `db`, `db-mysql`, `db-sqlite` |
| [`devtools`](/plugins/devtools) | Ops | `0.1.7` | In-app debug bar (HTML inject, SSE timeline, request snapshots) | `devtools`, `devtools-acme`, `devtools-csrf`, `devtools-fs`, `devtools-i18n`, `devtools-notifications`, `devtools-passport`, `devtools-rate-limit`, `devtools-redis`, `devtools-store` |
| [`env`](/plugins/env) | HTTP | `0.1.1` | Cascade .env loading for Sova apps (dotenvy) | `env` |
| [`fs`](/plugins/fs) | Data | `0.1.0` | Local filesystem with jail root (async CRUD + walk) | `fs` |
| [`http`](/plugins/http) | Integrations | `0.1.1` | Outbound HTTP client with SSRF guards and named configs | `http-client` |
| [`i18n`](/plugins/i18n) | Content | `0.1.2` | Locales, catalogs, optional path prefix and cookie | `i18n`, `i18n-cookie` |
| [`idempotency`](/plugins/idempotency) | HTTP | `0.1.1` | Replay 2xx responses for Idempotency-Key on mutating methods | `idempotency` |
| [`mail`](/plugins/mail) | Content | `0.1.2` | Outbound email via lettre (SMTP / fake / file) | `mail`, `mail-markdown`, `mail-templates` |
| [`meta`](/plugins/meta) | Content | `0.1.2` | Document meta, OG/Twitter, JSON-LD, and head inject | `meta`, `meta-i18n`, `meta-openapi`, `meta-store`, `meta-templates` |
| [`notifications`](/plugins/notifications) | Realtime | `0.1.5` | DB inbox, channels with ACL, optional WS/mail | `notifications`, `notifications-auth`, `notifications-mail`, `notifications-templates`, `notifications-ws` |
| [`observability`](/plugins/observability) | Ops | `0.1.2` | HTTP metrics, OpenTelemetry, Elasticsearch log shipping | `observability`, `observability-elasticsearch`, `observability-otel` |
| [`openapi`](/plugins/openapi) | Content | `0.1.1` | OpenAPI 3.1 document + Scalar UI at mount path | `openapi` |
| [`passport`](/plugins/passport) | Auth | `0.1.3` | Users + access/refresh JWT + personal access tokens | `passport`, `passport-jwt`, `passport-oauth`, `passport-session` |
| [`quic`](/plugins/quic) | Realtime | `0.1.2` | QUIC datagrams BackgroundService helpers for Sova | `quic-udp` |
| [`rate-limit`](/plugins/rate-limit) | HTTP | `0.1.3` | Per-key request rate limiting | `rate-limit` |
| [`redis`](/plugins/redis) | Data | `0.1.2` | Shared Redis/Valkey connection for KvStore, tasks, cache, pub/sub, queues | `redis` |
| [`response-cache`](/plugins/response-cache) | HTTP | `0.1.1` | Cache GET 200 responses in KvStore | `response-cache` |
| [`session`](/plugins/session) | Auth | `0.1.4` | Cookie sessions backed by a SessionStore | `session`, `session-redis`, `session-sql` |
| [`shield`](/plugins/shield) | HTTP | `0.1.1` | Baseline security response headers (helmet-style) | `shield` |
| [`sse`](/plugins/sse) | Realtime | `0.1.1` | Server-Sent Events helpers for Sova (channels, Last-Event-ID, keep-alive) | `sse-feed` |
| [`static`](/plugins/static) | HTTP | `0.1.1` | Serve files from a directory under a mount path | `static-files` |
| [`storage`](/plugins/storage) | Data | `0.1.1` | Object storage (local / memory / S3 / GCS / Azure) | `storage`, `storage-azure`, `storage-gcs`, `storage-memory`, `storage-s3` |
| [`store`](/plugins/store) | Data | `0.1.3` | KvStore trait + memory / file / sql / redis / redb backends for Sova | `store`, `store-crypto`, `store-file`, `store-redb`, `store-redis`, `store-sql` |
| [`tasks`](/plugins/tasks) | Data | `0.1.4` | Job worker, priorities, and optional cron/interval scheduler | `tasks`, `tasks-redis`, `tasks-sql` |
| [`tasks-store`](/plugins/tasks-store) | Data | `0.1.1` | TaskStore trait + memory / file / sql / redis backends | `tasks-file`, `tasks-redis`, `tasks-sql`, `tasks-store` |
| [`templates`](/plugins/templates) | Content | `0.1.1` | MiniJinja HTML templates with optional autoreload | `templates` |
| [`udp`](/plugins/udp) | Realtime | `0.1.1` | UDP BackgroundService helpers for Sova | `udp` |
| [`vld`](/plugins/vld) | Auth | `0.1.4` | Request validation hooks and coverage check | `vld`, `vld-flash`, `vld-flash-templates`, `vld-form`, `vld-i18n`, `vld-openapi` |
| [`ws`](/plugins/ws) | Realtime | `0.1.1` | WebSocket hub, origin allowlist, max message size | `ws` |
| [`cli`](/plugins/cli) | Tooling | `0.1.1` | CLI ServerArgs / listen_args for Sova (local dev) | — |
<!-- /generated:plugins-table -->

## Stack notes

### Auth (Fortify)

Needs **db + session**. Add **mail** only for `EmailVerification` / `ResetPasswords` (`auth-mail`).

```rust
app.install(Db::from_env().migrations::<AuthMigrator>());
app.install(memory_sessions()); // or SessionLayer::from_store(...)
app.install(Mail::from_env()); // when ResetPasswords / EmailVerification
app.install(
  Fortify::new()
    .features([AuthFeature::Registration, AuthFeature::ResetPasswords])
    .home("/cabinet"),
);
cabinet.use_middleware(Fortify::guard());
```

`auth-activity` → activity log. Full page: [auth](/plugins/auth).

### Passport (JWT / PAT / OAuth)

| Kind | Storage | Use |
|------|---------|-----|
| JWT access | signed, short TTL | browsers |
| Refresh | `auth_refresh_tokens` | rotate access |
| PAT (`svpat_…`) | `auth_api_tokens` | machine / CI |

`JwtAuth::guard` accepts Bearer JWT or PAT. OAuth: GitHub / Google / Apple / Custom (`{NAME}_CLIENT_ID` / `_CLIENT_SECRET`). → [passport](/plugins/passport).

### Tasks

Same handlers for Dispatch, CLI, and HTTP enqueue. Load `sova.toml` **before** `install(Tasks…)` so `[schedule.*]` applies.

```toml
[schedule.ping]
every = "15s"

[schedule.mail_digest]
cron = "0 */5 * * * *"
queue = "mailer"
priority = -100
```

Toml overrides code `.cron()` / `.every()` per job name (`cron` **or** `every`). CLI: `tasks list` | `schedule` | `run NAME [--json]`. Dispatch: `req.try_state::<TaskBackend>()` + `Dispatch::new(...)`. → `examples/misc/tasks`, [tasks](/plugins/tasks).

### Database

`DATABASE_URL` or `[db] url` (env wins when set). CLI: `migrate` / `seed` (`cargo sovax db …`). SQL KV/queue backends reuse `DbPool`. → [db](/plugins/db).
