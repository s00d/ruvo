//! Feature documentation for VitePress auto-generation.
//!
//! Lines must match: `/// Feature \`name\`: description`
//! Parsed by `sova-docs-gen`. Not a public API surface.

#![allow(dead_code)]

/// Feature `default`: Enables `static-files`.
/// Feature `static-files`: Serve a directory under a URL mount (`Static`).
/// Feature `cors`: Cross-origin headers / preflight (`Cors`).
/// Feature `csrf`: Double-submit CSRF + XSRF cookie (needs `session`).
/// Feature `shield`: Helmet-style security response headers.
/// Feature `cookies`: Parse `Cookie` header → `req.cookies()` + set-cookie helpers.
/// Feature `compress`: gzip / deflate / brotli response compression.
/// Feature `rate-limit`: Fixed-window rate limiting (memory or `KvStore`).
/// Feature `idempotency`: Inbound `Idempotency-Key` replay for mutating methods.
/// Feature `response-cache`: Cache public GET 200 responses in `KvStore`.
/// Feature `session`: Cookie sessions + flash (`SessionLayer` / `memory_sessions`).
/// Feature `session-sql`: Persist sessions in SQL via `DbPool`.
/// Feature `session-redis`: Persist sessions in Redis via `RedisPool`.
/// Feature `templates`: MiniJinja HTML templates (`req.render`).
/// Feature `multipart`: urlencoded / multipart `Request::input` + uploads.
/// Feature `cli`: `ServerArgs` / listen flags (`sovax` / `cargo-sovax`).
/// Feature `vld`: Typed request validation (`vld::schema!`, `req.validate`).
/// Feature `openapi`: OpenAPI 3.1 document + Scalar UI.
/// Feature `i18n`: Locales, catalogs, optional path prefix.
/// Feature `i18n-cookie`: Remember locale in a cookie.
/// Feature `ws`: WebSocket upgrade + rooms hub (`app.ws`).
/// Feature `store`: `KvStore` + `Cache` (sessions, CSRF, rate-limit, …).
/// Feature `store-file`: File-backed `KvStore`.
/// Feature `store-sql`: SQL `KvStore` on `DbPool`.
/// Feature `store-redis`: Redis `KvStore` on `RedisPool`.
/// Feature `store-crypto`: XChaCha20-Poly1305 wrapper for `KvStore`.
/// Feature `tasks-store`: `TaskStore` backends crate.
/// Feature `tasks-file`: File `TaskStore`.
/// Feature `tasks-sql`: SQL `TaskStore` on `DbPool`.
/// Feature `tasks-redis`: Redis `TaskStore` on `RedisPool`.
/// Feature `tasks`: Job queue, worker, scheduler, `tasks` CLI.
/// Feature `udp`: UDP `BackgroundService` (`UdpService`).
/// Feature `quic-udp`: QUIC datagrams (`QuicDatagramService`).
/// Feature `sse-feed`: SSE channel helpers (`SseChannel`, `sse_response`).
/// Feature `env`: Cascade `.env*` loader.
/// Feature `tls`: TLS terminate + optional HSTS/redirect.
/// Feature `dev-tls`: Self-signed local TLS via `rcgen`.
/// Feature `vld-openapi`: Validation ↔ OpenAPI schema sugar.
/// Feature `vld-flash`: Validation errors into session flash.
/// Feature `vld-flash-templates`: Flash helpers in MiniJinja.
/// Feature `vld-form`: Bind validation to multipart/form input.
/// Feature `vld-i18n`: Localized validation messages.
/// Feature `http-client`: Outbound HTTP client + SSRF guards (`req.http()`).
/// Feature `mail`: SMTP / fake / file mailer (`req.mail()`).
/// Feature `mail-templates`: MiniJinja mail bodies / Mailable views.
/// Feature `mail-markdown`: Markdown mail bodies.
/// Feature `ai`: Plugin + `req.ai()` + `FakeAi` (AISDK shell).
/// Feature `ai-openai`: OpenAI provider (`aisdk/openai`).
/// Feature `ai-anthropic`: Anthropic provider.
/// Feature `ai-google`: Google provider.
/// Feature `ai-full`: All aisdk providers.
/// Feature `ai-prompt`: File-based aisdk prompt templates.
/// Feature `storage`: Object storage (`req.storage()` — local / cloud).
/// Feature `storage-s3`: S3 / R2 / MinIO backend.
/// Feature `storage-gcs`: Google Cloud Storage backend.
/// Feature `storage-azure`: Azure Blob backend.
/// Feature `storage-memory`: In-memory blob store (tests).
/// Feature `passport`: Auth strategies registry (JWT / PAT / OAuth).
/// Feature `passport-session`: Session serialize/login for Passport.
/// Feature `passport-jwt`: JWT access + refresh + personal access tokens.
/// Feature `passport-oauth`: OAuth2 drivers (GitHub/Google/Apple/Custom).
/// Feature `auth`: Fortify — register/login/verify/reset/2FA/RBAC.
/// Feature `auth-mail`: Email verify/reset (needs `mail` + Fortify mail helpers).
/// Feature `auth-vld`: Fortify forms wired to `vld` flash/form.
/// Feature `activity`: Audit / activity log table + mount.
/// Feature `auth-activity`: Fortify mutations write activity events.
/// Feature `notifications`: DB inbox + named channels / ACL.
/// Feature `notifications-ws`: Push notifications over WebSocket.
/// Feature `notifications-mail`: Mail delivery channel.
/// Feature `notifications-auth`: Role/permission audiences.
/// Feature `notifications-templates`: Unread helpers in templates.
/// Feature `meta`: SEO head tags, Sitemap, Robots.
/// Feature `meta-openapi`: OpenAPI helpers for Meta routes.
/// Feature `meta-templates`: Inject meta into MiniJinja HTML.
/// Feature `meta-i18n`: Locale-aware meta.
/// Feature `meta-store`: Meta helpers backed by `KvStore`.
/// Feature `web`: HTML preset (cors, session, csrf, static, templates, meta, shield, cli, env, …).
/// Feature `api`: JSON API preset (cors, session, openapi, vld, cli, env, …).
/// Feature `db`: SeaORM pool (`req.db()`; postgres by default).
/// Feature `db-sqlite`: SQLite backend for `sova-db`.
/// Feature `db-mysql`: MySQL backend for `sova-db`.
/// Feature `observability`: Prometheus `/metrics` + HTTP RED middleware.
/// Feature `observability-otel`: OpenTelemetry OTLP export.
/// Feature `observability-elasticsearch`: Ship tracing logs to Elasticsearch.
/// Feature `devtools`: In-app DevTools bar (HTML inject + SSE timeline; auth/db/tasks soft-hooks).
/// Feature `devtools-store`: `devtools` + KvStore/Cache tracing (`sova.store`).
/// Feature `devtools-redis`: `devtools-store` + Redis messaging traces.
/// Feature `devtools-i18n`: locale soft-hook on snapshots.
/// Feature `devtools-csrf`: CSRF presence soft-hook.
/// Feature `devtools-passport`: Passport `Authenticated` soft-hook.
/// Feature `devtools-rate-limit`: rate-limit header soft-hook marker.
/// Feature `testing`: Expose `App::run_startup` / `run_shutdown` for tests.
/// Feature `listen-reuseport`: `SO_REUSEPORT` for graceful `cargo sovax dev`.
pub(crate) fn _doc_features_anchor() {}
