//! Feature documentation for VitePress auto-generation.
//!
//! Lines must match: `/// Feature \`name\`: description`
//! Parsed by `docs/scripts/generate.mjs`. Not a public API surface.

#![allow(dead_code)]

/// Feature `default`: Enables `static-files`.
/// Feature `static-files`: Serve static assets via `ruvo-static`.
/// Feature `cors`: CORS middleware (`ruvo-cors`).
/// Feature `csrf`: Session double-submit CSRF (`ruvo-csrf`; needs `session`).
/// Feature `shield`: Security response headers (`ruvo-shield`).
/// Feature `cookies`: Cookie jar helpers (`ruvo-cookies`).
/// Feature `compress`: gzip/deflate/brotli (`ruvo-compress`).
/// Feature `rate-limit`: Fixed-window rate limiting (`ruvo-rate-limit`).
/// Feature `session`: Cookie sessions + flash (`ruvo-session`).
/// Feature `session-sql`: Persist sessions in SQL via `DbPool`.
/// Feature `session-redis`: Persist sessions in Redis via `RedisPool`.
/// Feature `templates`: MiniJinja templates (`ruvo-templates`).
/// Feature `multipart`: Unified urlencoded/multipart `Request::input` / uploads.
/// Feature `cli`: `ServerArgs` and log CLI flags (`ruvo-cli`).
/// Feature `vld`: Request validation (`ruvo-vld` + `vld`).
/// Feature `openapi`: OpenAPI 3.1 + Scalar UI (`ruvo-openapi`).
/// Feature `i18n`: Locales and catalogs (`ruvo-i18n`).
/// Feature `i18n-cookie`: Remember locale in a cookie.
/// Feature `ws`: WebSocket upgrades (`ruvo-ws`).
/// Feature `store`: KvStore + Cache (`ruvo-store`).
/// Feature `store-file`: File-backed KvStore.
/// Feature `store-sql`: SQL KvStore on `DbPool`.
/// Feature `store-redis`: Redis KvStore on `RedisPool`.
/// Feature `store-crypto`: XChaCha20-Poly1305 wrapper for KvStore.
/// Feature `tasks-store`: TaskStore backends crate.
/// Feature `tasks-file`: File TaskStore.
/// Feature `tasks-sql`: SQL TaskStore on `DbPool`.
/// Feature `tasks-redis`: Redis TaskStore on `RedisPool`.
/// Feature `tasks`: Job queue, worker, scheduler, Console CLI (`ruvo-tasks`).
/// Feature `udp`: UDP `BackgroundService` (`ruvo-udp`).
/// Feature `quic-udp`: QUIC datagrams (`ruvo-quic`).
/// Feature `sse-feed`: SSE channel helpers (`ruvo-sse`).
/// Feature `env`: Cascade `.env*` loader (`ruvo-env`).
/// Feature `tls`: TLS terminate + optional HSTS/redirect (`ruvo-core/tls`).
/// Feature `dev-tls`: Self-signed local TLS via `rcgen`.
/// Feature `vld-openapi`: Validation ↔ OpenAPI schema sugar.
/// Feature `vld-flash`: Validation errors into session flash.
/// Feature `vld-flash-templates`: Flash helpers in MiniJinja.
/// Feature `vld-form`: Bind validation to multipart/form input.
/// Feature `vld-i18n`: Localized validation messages.
/// Feature `http-client`: Outbound HTTP (`ruvo-http`).
/// Feature `mail`: SMTP / fake / file mailer (`ruvo-mail`).
/// Feature `mail-templates`: MiniJinja mail bodies / Mailable views.
/// Feature `mail-markdown`: Markdown mail bodies.
/// Feature `storage`: Object storage (`ruvo-storage`).
/// Feature `storage-s3`: S3 / R2 / MinIO backend.
/// Feature `storage-gcs`: Google Cloud Storage backend.
/// Feature `storage-azure`: Azure Blob backend.
/// Feature `storage-memory`: In-memory blob store (tests).
/// Feature `passport`: Auth strategies registry (`ruvo-passport`).
/// Feature `passport-session`: Session serialize/login for Passport.
/// Feature `passport-jwt`: JWT access + refresh + PAT.
/// Feature `passport-oauth`: OAuth2 drivers (GitHub/Google/Apple/Custom).
/// Feature `auth`: Fortify (register/login/verify/reset/2FA/RBAC).
/// Feature `auth-vld`: Fortify forms wired to `vld` flash/form.
/// Feature `activity`: Audit / activity log table (`ruvo-activity`).
/// Feature `auth-activity`: Fortify mutations write activity events.
/// Feature `notifications`: DB inbox + channels (`ruvo-notifications`).
/// Feature `notifications-ws`: Push notifications over WebSocket.
/// Feature `notifications-mail`: Mail delivery channel.
/// Feature `notifications-auth`: Role/permission audiences.
/// Feature `notifications-templates`: Unread helpers in templates.
/// Feature `meta`: SEO head tags, Sitemap, Robots (`ruvo-meta`).
/// Feature `meta-templates`: Inject meta into MiniJinja HTML.
/// Feature `meta-i18n`: Locale-aware meta.
/// Feature `meta-store`: Meta helpers backed by KvStore.
/// Feature `web`: Preset for HTML apps (cors, session, csrf, static, templates, meta, shield, cli, env, reuseport).
/// Feature `api`: Preset for JSON APIs (cors, session, openapi, vld, cli, env, reuseport).
/// Feature `db`: SeaORM pool (`ruvo-db`; postgres by default).
/// Feature `db-sqlite`: SQLite backend for `ruvo-db`.
/// Feature `db-mysql`: MySQL backend for `ruvo-db`.
/// Feature `observability`: Prometheus `/metrics`.
/// Feature `observability-otel`: OpenTelemetry OTLP export.
/// Feature `observability-elasticsearch`: Ship tracing logs to Elasticsearch.
/// Feature `redis`: Shared Redis/Valkey pool (`ruvo-redis`).
/// Feature `testing`: Expose `App::run_startup` / `run_shutdown` for tests.
/// Feature `listen-reuseport`: `SO_REUSEPORT` for graceful `cargo ruvo dev`.
pub(crate) fn _doc_features_anchor() {}
