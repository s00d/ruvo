//! Feature documentation for VitePress auto-generation.
//!
//! Lines must match: `/// Feature \`name\`: description`
//! Parsed by `docs/scripts/generate.mjs`. Not a public API surface.

#![allow(dead_code)]

/// Feature `default`: Enables `static-files`.
/// Feature `static-files`: Serve static assets via `sova_static`.
/// Feature `cors`: CORS middleware (`sova_cors`).
/// Feature `csrf`: Session double-submit CSRF (`sova_csrf`; needs `session`).
/// Feature `shield`: Security response headers (`sova_shield`).
/// Feature `cookies`: Cookie jar helpers (`sova_cookies`).
/// Feature `compress`: gzip/deflate/brotli (`sova-compress`).
/// Feature `rate-limit`: Fixed-window rate limiting (`sova-rate-limit`).
/// Feature `session`: Cookie sessions + flash (`sova_session`).
/// Feature `session-sql`: Persist sessions in SQL via `DbPool`.
/// Feature `session-redis`: Persist sessions in Redis via `RedisPool`.
/// Feature `templates`: MiniJinja templates (`sova-templates`).
/// Feature `multipart`: Unified urlencoded/multipart `Request::input` / uploads.
/// Feature `cli`: `ServerArgs` and log CLI flags (`sovax`).
/// Feature `vld`: Request validation (`sova_vld` + `vld`).
/// Feature `openapi`: OpenAPI 3.1 + Scalar UI (`sova-openapi`).
/// Feature `i18n`: Locales and catalogs (`sova-i18n`).
/// Feature `i18n-cookie`: Remember locale in a cookie.
/// Feature `ws`: WebSocket upgrades (`sova_ws`).
/// Feature `store`: KvStore + Cache (`sova_store`).
/// Feature `store-file`: File-backed KvStore.
/// Feature `store-sql`: SQL KvStore on `DbPool`.
/// Feature `store-redis`: Redis KvStore on `RedisPool`.
/// Feature `store-crypto`: XChaCha20-Poly1305 wrapper for KvStore.
/// Feature `tasks-store`: TaskStore backends crate.
/// Feature `tasks-file`: File TaskStore.
/// Feature `tasks-sql`: SQL TaskStore on `DbPool`.
/// Feature `tasks-redis`: Redis TaskStore on `RedisPool`.
/// Feature `tasks`: Job queue, worker, scheduler, Console CLI (`sova_tasks`).
/// Feature `udp`: UDP `BackgroundService` (`sova_udp`).
/// Feature `quic-udp`: QUIC datagrams (`sova_quic`).
/// Feature `sse-feed`: SSE channel helpers (`sova_sse`).
/// Feature `env`: Cascade `.env*` loader (`sova-env`).
/// Feature `tls`: TLS terminate + optional HSTS/redirect (`sova-core/tls`).
/// Feature `dev-tls`: Self-signed local TLS via `rcgen`.
/// Feature `vld-openapi`: Validation ↔ OpenAPI schema sugar.
/// Feature `vld-flash`: Validation errors into session flash.
/// Feature `vld-flash-templates`: Flash helpers in MiniJinja.
/// Feature `vld-form`: Bind validation to multipart/form input.
/// Feature `vld-i18n`: Localized validation messages.
/// Feature `http-client`: Outbound HTTP (`sova_http`).
/// Feature `mail`: SMTP / fake / file mailer (`sova_mail`).
/// Feature `mail-templates`: MiniJinja mail bodies / Mailable views.
/// Feature `mail-markdown`: Markdown mail bodies.
/// Feature `ai`: AISDK language models (`sova_ai`).
/// Feature `ai-openai`: OpenAI provider via aisdk.
/// Feature `ai-anthropic`: Anthropic provider via aisdk.
/// Feature `ai-google`: Google provider via aisdk.
/// Feature `ai-full`: All aisdk providers (`aisdk/full`).
/// Feature `ai-prompt`: File-based aisdk prompt templates.
/// Feature `storage`: Object storage (`sova_storage`).
/// Feature `storage-s3`: S3 / R2 / MinIO backend.
/// Feature `storage-gcs`: Google Cloud Storage backend.
/// Feature `storage-azure`: Azure Blob backend.
/// Feature `storage-memory`: In-memory blob store (tests).
/// Feature `passport`: Auth strategies registry (`sova-passport`).
/// Feature `passport-session`: Session serialize/login for Passport.
/// Feature `passport-jwt`: JWT access + refresh + PAT.
/// Feature `passport-oauth`: OAuth2 drivers (GitHub/Google/Apple/Custom).
/// Feature `auth`: Fortify (register/login/verify/reset/2FA/RBAC).
/// Feature `auth-mail`: Email verify/reset templates (`mail-templates` + Fortify mail helpers).
/// Feature `auth-vld`: Fortify forms wired to `vld` flash/form.
/// Feature `activity`: Audit / activity log table (`sova-activity`).
/// Feature `auth-activity`: Fortify mutations write activity events.
/// Feature `notifications`: DB inbox + channels (`sova-notifications`).
/// Feature `notifications-ws`: Push notifications over WebSocket.
/// Feature `notifications-mail`: Mail delivery channel.
/// Feature `notifications-auth`: Role/permission audiences.
/// Feature `notifications-templates`: Unread helpers in templates.
/// Feature `meta`: SEO head tags, Sitemap, Robots (`sova_meta`).
/// Feature `meta-openapi`: OpenAPI helpers for Meta routes.
/// Feature `meta-templates`: Inject meta into MiniJinja HTML.
/// Feature `meta-i18n`: Locale-aware meta.
/// Feature `meta-store`: Meta helpers backed by KvStore.
/// Feature `web`: Preset for HTML apps (cors, session, csrf, static, templates, meta, shield, cli, env, reuseport).
/// Feature `api`: Preset for JSON APIs (cors, session, openapi, vld, cli, env, reuseport).
/// Feature `db`: SeaORM pool (`sova-db`; postgres by default).
/// Feature `db-sqlite`: SQLite backend for `sova-db`.
/// Feature `db-mysql`: MySQL backend for `sova-db`.
/// Feature `observability`: Prometheus `/metrics`.
/// Feature `observability-otel`: OpenTelemetry OTLP export.
/// Feature `observability-elasticsearch`: Ship tracing logs to Elasticsearch.
/// Feature `redis`: Shared Redis/Valkey pool (`sova_redis`).
/// Feature `testing`: Expose `App::run_startup` / `run_shutdown` for tests.
/// Feature `listen-reuseport`: `SO_REUSEPORT` for graceful `cargo sovax dev`.
pub(crate) fn _doc_features_anchor() {}
