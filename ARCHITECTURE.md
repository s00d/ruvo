# Architecture

Ruvo is a small Express-like HTTP framework: `ruvo-core` owns the request path;
plugins add optional middleware and helpers.

## Request path

```text
accept
  → server/conn (semaphore, JoinSet, hyper auto HTTP/1.1+HTTP/2 + with_upgrades)
  → to_ruvo_request (+ optional OnUpgrade)
  → CompiledRouter::dispatch
  → root middleware (onion)  // request_id → Observability → logger → …
  → matchit route match (+ MatchedRoute / MatchedRouteCapture)
  → route / mount middleware
  → handler
  → IntoResponse
  → hyper response body
```

### Observability

- Core: `request_id()` + structured `logger()`; type `RequestId`.
- Plugin `ruvo-observability` (features `observability` / `observability-otel` / `observability-elasticsearch`): Prometheus scrape `/metrics`, optional OTLP, optional Elasticsearch log shipping (`ELASTICSEARCH_URL` → `_bulk`).

`App::build()` compiles routes once into a cheap-to-clone [`Server`]. Prefer
`Server::handle` (or `handle_request`) in tests and embedded use so the matcher
is not rebuilt per request. `Server::state` / `Server::run_startup` (feature
`testing`) are non-destructive.

`handle_request(method, path, body)` is thin sugar over `Request::builder` with
**no custom headers**. For cookies, `Content-Type`, etc. use
`Request::builder().header(...).build()` and `handle`.

## Configuration (`ruvo.toml`)

One document for app limits, plugin defaults, and cargo-ruvo frontend. Secrets
stay in process env (often via `ruvo-env` + `.env*`).

```toml
[server]
max_body = "2mb"
trust_proxy = false

[mail]
from = "App <noreply@example.com>"

[storage]
driver = "local"
path = "storage"
public_url = "/storage"

[db]
url = "postgres://postgres@localhost/ruvo"

[redis]
url = "redis://127.0.0.1:6379"

# Optional: schedule registered jobs by name (overrides code .cron/.every)
# [schedule.digest]
# cron = "0 8 * * *"
# [schedule.cleanup]
# every = "30m"

[observability]
metrics_path = "/metrics"
otel = false
elasticsearch = false

[meta]
site_name = "My App"
public_url = "http://127.0.0.1:3000"

[frontend]          # cargo-ruvo only
dir = "frontend"

[development.server]
max_connections = 32

[production.server]
trust_proxy = true
```

**Profile:** `RUVO_PROFILE` → else `RUVO_ENV` → else `development` (debug builds) /
`production` (release). Aliases: `debug`→`development`, `release`→`production`.
`cargo ruvo dev` sets `RUVO_ENV=development`; `serve` sets `production`.

**Merge order:** built-in defaults → `[section]` / legacy `[default.section]` →
`[<profile>.section]` → `RUVO_*` / plugin env (secrets, URLs) → explicit builder
methods in code (unset-fill: toml only fills fields you did not set on the builder).

**Load:** `App::configure()` (cwd `ruvo.toml` / `Ruvo.toml`) or
`configure_from_path`. Presets `App::web()` / `App::api()` call `ruvo_env::load()`
then `configure()` automatically. Raw `App::new()` must call configure itself.

**Do not put in toml:** code-shaped settings (Fortify feature bits, RateLimit key
fns, Notification channels, OAuth provider matrices, job handlers).

See `examples/cabinet/ruvo.toml` for a full kitchen-sink sample.

## Lifecycle

```text
start: compile → on_startup → BackgroundServices → accept
stop:  stop accept → drain connections → stop services → on_shutdown
```

`app.run()` is the primary entrypoint: it parses CLI commands (`check`, `routes`,
`openapi --out`, `tasks`, `i18n missing`, plus plugin-registered commands such as
`migrate` / `seed`) and exits, or starts the server path.
CLI command mode runs startup/shutdown hooks and skips the accept loop.

### Health probes

`App::with_probes()` (default on `App::web` / `App::api`) installs:

| Endpoint | Role | Behavior |
|----------|------|----------|
| `GET /healthz` | liveness | `200` + `{"status":"ok"}` — process up; no plugin checks |
| `GET /ready` | readiness | runs `CheckKind::Ready` checks; all ok → `200`, else `503` + failed names |

`register_check` → Ready (db/redis/storage/tasks/kv). `register_audit` → Audit
(openapi/vld/templates/meta/i18n) — CLI `check` only, not `/ready`.
CLI `myapp check` runs Ready **and** Audit.

### JWT + PAT (feature `passport-jwt`)

`JwtAuth` installs register/login/refresh/logout and (by default) PAT CRUD at
`/auth/tokens`. `JwtAuth::guard` accepts `Authorization: Bearer` for either a
short-lived JWT access token or a personal access token (`rvpat_<prefix>_<secret>`).

| Kind | Storage | Use |
|------|---------|-----|
| JWT access | signed, short TTL | browsers / interactive |
| Refresh | `auth_refresh_tokens` (SHA-256) | rotate access |
| PAT | `auth_api_tokens` (SHA-256 + prefix) | machine clients / CI |
| `Auth::api_key` | your verify callback | ad-hoc keys, no DB |

PAT plaintext is returned **once** on `POST /auth/tokens`. List never includes it.
Abilities: JSON string array; `[]` = full access (`token_can`).

### Activity log (feature `activity` / `auth-activity`)

Laravel-style **who-changed-what** via `ruvo-activity` (not HTTP request logging).

| Piece | Role |
|-------|------|
| `ActivityMigrator` | table `activity_log` (+ indexes on subject, actor, event, created_at) |
| `Activity::new()` | state; optional `.mount("/activity")` → `GET` list |
| `req.log_activity` / `ActivityLog::record` | best-effort insert (`tracing::warn` on failure; never fails the business op) |
| `auth-activity` | Fortify actions write events after successful mutations |

Fortify events (feature `ruvo-auth/activity`): `user.registered`, `user.login` / `user.logout`, `profile.updated`, `password.changed`, `email.verified`, `2fa.enabled` / `2fa.confirmed` / `2fa.disabled`, `role.*`, `permission.*`, `role.permissions.synced`, `user.roles.synced`. Properties hold old/new field diffs — **never** passwords, hashes, or secrets.

```rust
app.install(Db::from_env().migrations::<CombinedMigrator>()); // Auth + Activity
app.install(Activity::new().mount("/activity").guard(Fortify::permission("users.manage")));
```

Do **not** middleware-log every request; call `Activity` from app code or enable Fortify wiring.

### Notifications (feature `notifications`)

DB inbox (not vld flash). Named **channels** with optional publish/subscribe permission slugs.

| Piece | Role |
|-------|------|
| `NotificationsMigrator` | table `notifications` |
| `Notifications::new().channel(…).mount(…).guard(…)` | HTTP inbox + broadcast |
| `Notify::to` / `to_many` / `to_role` / `to_permission` | send (auth feature for role/permission audience) |
| `Via::Database` / `Via::Ws` / `Via::Mail` | delivery (ws/mail features) |
| `notifications_unread` | template helper (with `preload_unread` / `notifications-templates`) |

System `Notify::…send` skips publish ACL; `POST /notifications/broadcast` uses `.as_user()` and enforces channel publish permission. List/mark always scoped to current user.

```rust
app.install(
  Notifications::new()
    .channel(Channel::new("orders").publish("notifications.orders.publish"))
    .mount("/notifications")
    .guard(Fortify::guard())
    .ws_path("/ws/notifications")
);
Notify::to(uid).channel("orders").event("order.shipped").title("Shipped").send(&req).await?;
```

### Rate limit (feature `rate-limit`)

Default key is client IP. Prefer identity for authenticated traffic:

| Key | Behavior |
|-----|----------|
| `RateLimitKey::Ip` | `ClientAddr` (default) |
| `RateLimitKey::Identity` | `RateLimitIdentity` (set by Passport/Fortify) else IP |
| `RateLimitKey::IpAndInput { field }` | IP + form/json field (login email) |

Login throttling is a **separate preset**, not part of `WebApp`/`ApiApp`:

- `RateLimit::login()` / `forgot()` / `challenge()` / `resend()` — route MW via `.middleware()`
- Fortify installs these on auth POSTs (IP+email for login/forgot/resend)
- Global app limit stays opt-in: `app.install(RateLimit::per_minute(120).key(RateLimitKey::Identity))`

### OAuth (feature `passport-oauth`)

Drivers (one file each under `plugins/ruvo-passport/src/oauth/drivers/`):
`Github` / `Google` / `Apple` / `Custom` implement [`Driver`] (`client_id`, `scopes`,
`from_env`, `build`, …). Install with `Oauth::provider(Github::new().from_env())`
(`provider` takes `impl Into<OauthProvider>`).

`from_env` loads `{NAME}_CLIENT_ID` / `{NAME}_CLIENT_SECRET` / optional `{NAME}_REDIRECT_URI`.
Apple also uses `APPLE_TEAM_ID`, `APPLE_KEY_ID`, `APPLE_PRIVATE_KEY` (PEM) to mint the
client_secret JWT. Common: `OAUTH_PUBLIC_URL`, `OAUTH_STATE_SECRET` (fallback `JWT_SECRET`).

Routes: `GET {mount}/{name}` → IdP; `GET|POST {mount}/{name}/callback` (Apple uses
`response_mode=form_post`). Google preset sends `access_type=offline` + `prompt=consent`.

Plugin errors that already know their HTTP shape use [`Error::Response`] /
[`Error::custom`]; `wrap_errors` does **not** pass them through `error_handler`.
Validation (`ruvo-vld`) and SeaORM `DbErr` convert into that bridge so handlers
stay on `ruvo_core::Result` with `?`.

## Database (plugin `ruvo-db`, feature `db`)

SeaORM pool per process; backend via Cargo features (`postgres` default, `sqlite`, `mysql`)
and `DATABASE_URL`:

```text
Db::from_env() → on_startup connect+ping → req.db() → ConnectionTrait
transaction() middleware → commit on 2xx / rollback otherwise
Db::migrations::<M>() → myapp migrate [up|down|status] [N]
Db::seed(fn) → myapp seed (explicit; not on server start)
cargo ruvo db migrate|down|status|seed → cargo run -p <pkg> -- …
```

`migrate status` prints a human-readable Applied/Pending table to stdout (not only tracing).
`migrate down N` rolls back N steps (default 1).

Scaffolding (`cargo ruvo generate`): `mailer`, `job`/`worker`, `migration`, `seed`, `resource` (web default, `--api` for JSON; `crud` aliases `--api`), plus existing `module` / `plugin` / `model`.

SQL KV / queue backends (`store-sql`, `tasks-sql` → `SqlStore` / `SqlTaskStore`)
reuse the same [`DbPool`] — no second connection string. Custom drivers: `impl KvStore`
/ `impl TaskStore` + `SharedStore::new` / `Tasks::new`.

## Tasks scheduler (plugin `ruvo-tasks`)

Jobs unify handler + optional schedule/queue/priority; worker and scheduler share one `TaskStore`:

```text
Tasks::new(store)
  .queues(["critical", "default", "mailer"])  // claim order = queue priority
  .job(Job::new("cleanup", handler).every(Duration::from_secs(3600)))
  .job(Job::new("welcome_email", handler).queue("mailer").priority(priority::LOW))

tasks.dispatch(Dispatch::new("welcome_email").data(json!({…})))
tasks.dispatch(Dispatch::new("report").delay(Duration::from_secs(30)).priority(priority::HIGH))
```

**CLI** (after `Tasks` is installed): `tasks list` | `tasks schedule` |
`tasks run NAME [--json '{…}']`. During `tasks run`, handlers may use Console IO
(`info` / `table` / `ask` / `confirm`); Dispatch / worker paths keep Console off
(`ask` errors, `confirm` returns default, prints are no-op).

**`[schedule.<job>]` in `ruvo.toml`:** key = registered job name; set `cron` **or**
`every` (optional `queue` / `priority`). Toml **overrides** code `.cron()`/`.every()`.
Unknown names fail startup + audit check `tasks-schedule`.

Within a queue, claim sorts by **priority DESC**, then `run_at`. Scheduler is a second
`BackgroundService` when any job has `.every`/`.cron`. Multi-instance safe via `dedup_key` slots.

## Redis (plugin `ruvo-redis`, features `redis` / `store-redis` / `tasks-redis`)

Shared [`RedisPool`] (like `DbPool`) for horizontal scale — same trait surfaces:

```text
Redis::from_env() → REDIS_URL (else `[redis] url`) → on_startup connect+PING → req.redis()
store::Redis::from_redis_pool(&pool)  → KvStore (rate-limit, Cache)
RedisSessionStore::from_redis_pool(&pool) → SessionStore (feature session-redis)
tasks::Redis::from_redis_pool(&pool) → TaskStore (lease claim via Lua)
AppStore::cache() → JSON get/set/remember over any KvStore
pool.publish / subscribe / psubscribe → Pub/Sub
pool.enqueue / dequeue / dequeue_wait → list queues (LPUSH / RPOP / BRPOP)
```

Valkey / KeyDB work via the Redis protocol. Cabinet stays on SQL; see `examples/misc/redis`.

## Object storage (plugin `ruvo-storage`, feature `storage`)

Unified `BlobStore` (put/get/delete/exists/list + optional temporary URLs) + `AppStorage`:

```text
Storage::local(path).public_url("/assets/uploads")
Storage::memory() | Storage::s3_from_env() | … | Storage::from_env()
req.storage().put_upload(key, &upload)
req.storage().list("avatars/")
req.storage().temporary_url(key, Duration::from_secs(300))  // s3/gcs/azure
```

`PutOpts`: `content_type`, `content_disposition`, `cache_control`, `metadata` (user metadata on cloud).

Backends via features: `local`/`memory` (default), `storage-s3` (S3/R2/MinIO),
`storage-gcs`, `storage-azure` (OpenDAL). Custom: `impl BlobStore` + `Storage::new`.

`RUVO_STORAGE=local|memory|s3|gcs|azure` selects the backend for `Storage::from_env()`.
Common: `RUVO_STORAGE_PUBLIC_URL`, `RUVO_STORAGE_ROOT` (key prefix), `RUVO_STORAGE_PATH` (local root).

| Backend | Env |
|---------|-----|
| **local** | `RUVO_STORAGE_PATH` (default `./storage`) |
| **memory** | (none) |
| **s3** | `RUVO_STORAGE_BUCKET` / `AWS_BUCKET`; `RUVO_STORAGE_REGION` / `AWS_REGION` (required unless `RUVO_STORAGE_ENDPOINT` → region `auto`); `RUVO_STORAGE_ENDPOINT` (R2/MinIO); `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`; `AWS_SESSION_TOKEN`; `RUVO_STORAGE_FORCE_PATH_STYLE` (`1`/`0`; default path-style when endpoint set, virtual-host for plain AWS) |
| **gcs** | `RUVO_STORAGE_BUCKET`; `GOOGLE_APPLICATION_CREDENTIALS`; optional `RUVO_STORAGE_ROOT` / `RUVO_STORAGE_ENDPOINT` |
| **azure** | `RUVO_STORAGE_CONTAINER` or `RUVO_STORAGE_BUCKET`; `AZURE_STORAGE_ACCOUNT_NAME` / `AZURE_STORAGE_ACCOUNT_KEY`; `AZURE_STORAGE_ENDPOINT` or `RUVO_STORAGE_ENDPOINT` (Azurite) |

MinIO / R2 demo: `examples/misc/storage`.

## Mail (plugin `ruvo-mail`, feature `mail`)

`Mail::try_from_env() -> Result<Mail>` / `Mail::from_env()` (panics on bad SMTP URL).

- `RUVO_MAIL` / `RUVO_MAIL_MAILER`: `fake` | `smtp` | `file` (optional)
- No mailer + no URL → **fake** (dev)
- Invalid `RUVO_MAIL_URL` / `SMTP_URL` → **Err** (never silent fake)
- `file` → `RUVO_MAIL_PATH` (default `./mail`)
- Bodies: `.view("mail/welcome.html", ctx)` with feature `mail-templates`
- Markdown: `.markdown("# Hi")` / `.markdown_view("mail/note.md", ctx)` with `mail-markdown`
- Mailable: `impl Mailable` + `req.mail().to(u).send_mail(Welcome { .. }).await?`
- Auth verify/reset: `VerifyEmailMail` / `ResetPasswordMail` → views `mail/verify.html` / `mail/reset.html`

## TLS (feature `tls`)

TLS terminates in the accept task (not the accept loop): `TcpStream` →
`tokio-rustls` handshake (with timeout) → generic IO into hyper auto (HTTP/1.1+HTTP/2).
`OnUpgrade` / WebSocket need no TLS-specific code (WSS is WS over TLS).

Hot reload: `ResolvesServerCert` + `ArcSwap<CertifiedKey>` — call
`TlsRuntime::reload()` or replace PEM on disk and reload. Optional
`.redirect_http(80)` and `.hsts(true)`.

`dev-tls` uses `rcgen` for local self-signed certs.

## Env (plugin `ruvo-env`)

Call `ruvo_env::load()` explicitly at the top of `main` (never inside
`App::new()`). Mode: `RUVO_ENV` / `APP_ENV` / debug→`development` /
release→`production` / test→`test`. File aliases: `development`→`.env.dev`,
`production`→`.env.prod`.

Cascade (later file wins; process env is never overwritten):

1. `.env.{short}` (`.env.dev` / `.env.prod` / `.env.test`)
2. `.env.{mode}` when mode ≠ short (e.g. `.env.development`)
3. `.env.local` (skipped in `test`)
4. `.env` (final overlay)

**URLs:** `[db] url` / `[redis] url` in toml; non-empty `DATABASE_URL` /
`REDIS_URL` override. No `url_env` key.

## Store encryption (feature `store-crypto`)

`Encrypted<S: KvStore>` wraps any backend; values are XChaCha20-Poly1305,
keys use stable HMAC tails for `clear_prefix`. `incr` bypasses encryption.

## UDP

Plain datagrams only — no DTLS. Encrypted datagrams: future `ruvo-quic`.

## Long-lived connections

| Kind | How |
|------|-----|
| HTTP upgrade (WS) | Route handler + `req.on_upgrade()`; budget via `max_upgraded_connections` → **503** + `Retry-After` |
| SSE / stream body | Normal `Response`; `request_timeout` ends when the handler **returns** — stream chunks after that are not cut by it |
| Raw Hyper | `Router::raw` — **last resort**; skips middleware |

Upgraded / service state is **process-local** (not shared across processes).

## Public surface: root vs `extend`

| Surface | Audience | Examples |
|---------|----------|----------|
| **crate root** | Applications | `App`, `Server`, `Router`, `Request`, `Response`, typed bodies (`Html`/`Json`/…), `Error`/`Result`/`IntoResponse`, `Plugin`, `Next`, `logger`, `with_state`, `ClientAddr`, `BackgroundService`, `OnUpgrade`, `Cell`, `Slot`, `LogConfig`/`LogRotate` |
| **`extend`** | Plugins / advanced | `Handler`/`IntoHandler`, `ErrorResponse`, middleware traits, `named`/`with_leaked`, bodies (`Body`, `HttpBody`), path helpers, `RouteEntry`/`RouteTable`, `Extensions`/`TypeMap` (`StateMap` alias), `MatchedMeta`, `RequestBuilder`, `wait_shutdown`, `Cell`, `Slot` |

Route metadata is a [`TypeMap`](crates/ruvo-core/src/state.rs) on each HTTP route (`route_meta`).
**Same type twice — last wins**; different types never conflict. After a match,
the bag is available on the request as `MatchedMeta` / `req.route_meta::<T>()`.

The `ruvo` facade re-exports the same root list and `ruvo::extend`.

## Ownership

| Layer | Owns |
|-------|------|
| **ruvo-core** | App/Router/Server, dispatch, Request/Response, middleware traits, listen/drain, `ClientAddr`, route `TypeMap`, `BackgroundService`, `OnUpgrade`, `Cell`/`Slot` (cross-task share) |
| **ruvo-core** (+ feature `multipart`) | Unified request input: urlencoded / multipart → `Request::input` / `form` / `Upload::save`; responses `file` / `download` |
| **plugins** | Optional features: cookies, session / session-sql / session-redis, observability / observability-otel, csrf, rate-limit, cors, compress, static, templates, vld, openapi, i18n, ws, tasks, store, storage (local/S3/GCS/Azure), udp, sse, mail / mail-templates, passport / passport-jwt / passport-oauth, auth / auth-activity, activity, notifications, http-client, **db**, **redis**, store-sql, tasks-sql |

Plugins depend on `ruvo_core` (and sometimes other plugins). Core does not depend on plugins. **KvStore is not in core** — wire via `app.state(...)`.

## Tests

| Location | Scope |
|----------|--------|
| `src/**` `#[cfg(test)]` | Unit tests of **private** helpers (`collect_limited`, path normalize, …) |
| `crates/*/tests/` and `plugins/*/tests/` | Integration against the **public** API (`root` + `extend`) |
| **`ruvo-testing`** | Shared harness: tempfile sqlite + migrate, `TestApp`, `acting_as` / `acting_as_id`, `UserFactory`, `ResponseAssert`, insta JSON snapshots |

`TestClient` (cookie jar + `on_request` hooks) lives in **ruvo-core**. Feature `testing` exposes `App::run_startup` / `run_shutdown` for lifecycle tests
(`cargo test --features testing` or `--all-features`).
Plugin integration tests should take `ruvo-testing` as a **dev-dependency** (facade `testing` does not pull the full harness).

`ruvo-db::test_db` / `TestDb` remains for optional external-`DATABASE_URL` + transaction rollback; sqlite tempfile via `SqliteTestDb` / `TestApp` is the default for plugin tests.

Feature `listen-reuseport` enables `BoundApp::reuseport(true)` (`SO_REUSEPORT`). Also enabled via env `RUVO_REUSEPORT=1` (pulled in by facade `web` / `api`).

### Hot reload

| Layer | Mechanism |
|-------|-----------|
| Templates / i18n catalogs | In-process FS watch (no process restart) |
| Code (`.rs`), `.env*`, `ruvo.toml` | `cargo ruvo dev` process restart |
| Graceful (default on Unix) | Spawn new with `RUVO_REUSEPORT` + `RUVO_INSTANCE_ID` → wait `GET /ready` with matching `x-ruvo-instance` → SIGTERM old → drain → SIGKILL after `--drain-timeout` |

WebSocket / upgraded connections stay on the old process until drain; they are not migrated.
