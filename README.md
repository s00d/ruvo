# Ruvo

Express-like HTTP for Rust: `App`, `Router`, middleware, plugins — Hyper stays hidden.

## Quick start (web)

```rust
use ruvo::prelude::*;
use ruvo::{Html, Meta};

#[tokio::main]
async fn main() -> Result<()> {
    App::web()
        .site("Blog")
        .public_url("https://example.com")
        .listen(3000)
        .await
}
```

Enable with `features = ["web"]`. Routes still work via `DerefMut`:

```rust
let mut app = App::web().site("Blog").public_url("https://example.com");
app.get("/about", || async { Html("<h1>About</h1>".into()) })
    .with(Meta::page().title("About").description("…"));
app.listen(3000).await
```

Head tags are injected automatically for HTML responses. `App::web()` also installs `Sitemap` and `Robots` (`/sitemap.xml`, `/robots.txt`). For APIs use `App::api()` (`features = ["api"]`).

```rust
app.install(Sitemap::new().exclude("/api/*").include("/app"));
app.install(Robots::new().disallow("/admin"));
```

## Manual stack

```rust
use ruvo::prelude::*;
use ruvo::Cors;

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.use_middleware(request_id());
    app.install(Observability::new()); // feature `observability` → GET /metrics
    app.use_middleware(logger());
    app.install(Cors::new().origin("*"));

    app.get("/", home);
    app.with_probes();
    modules::register(&mut app);

    app.listen(3000).await
}
```

`request_id()` accepts/generates `x-request-id`, echoes it on the response, and opens an `http.server` span (presets `App::web()` / `App::api()` install it before `logger()`).

`App::with_probes()` installs `GET /healthz` (liveness) and `GET /ready` (readiness via `register_check`). Presets enable probes automatically. Deploy-time audits use `register_audit` and run only under CLI `myapp check`.

Redirects: handler `Redirect::to("/x")` / `Response::redirect`, or route helper `app.redirect("/old", "/new", 302)` (status is the 3rd arg).

Use `ruvo::{Cors, Csrf, Static, Meta, Sitemap, Robots, …}` for plugins; everyday names come from [`prelude`](crates/ruvo/src/lib.rs).

## Request input

With feature `multipart` (urlencoded works without it for classic forms):

```rust
// urlencoded or multipart — one parse, cached on the request
let data = req.input().await?;
let title = data.get("title");
let file = data.file("avatar").cloned().unwrap();
file.validate(&UploadRules::new().max_bytes(2_000_000).extensions(["png", "jpg"]))?;
let stored = req.storage().store(&file, "avatars").await?; // StoredFile { key, url }

// text fields → struct (both encodings)
let body: CreatePost = req.form().await?;

// download helper (file + Content-Disposition: attachment)
Response::download("report.pdf").await
// status: Response::json(&x).status(201) or (201, Json(x))
```

`req.json()` is unchanged. There is no separate multipart crate.

### Flash / redirect / paginate / login

```rust
req.flash_status("Saved");
req.flash_old(&json!({ "title": "…" }));
Ok(Redirect::back(&req).into_response()) // Referer or "/"

// templates via with_flash(...): {{ status }}, {{ errors.field }}, {{ old.title }}

let page = Entity::find()
    .filter(...)
    .paginate_ruvo(req.db(), req.page_params())
    .await?;
// page.data / page.total / page.page / page.last_page

// Programmatic session auth (Fortify CurrentUser):
req.login_user(cu);   // regenerate sid + passport:user + SessionStore user index
req.logout_user();
req.logout_other_sessions().await?; // other devices only
req.logout_all_sessions().await?;   // including current cookie
// SQL sessions: SessionLayer::from_store(Arc::new(SqlSessionStore::from_db_pool(&pool)))
// Redis: SessionLayer::from_store(Arc::new(RedisSessionStore::from_redis_pool(&pool)))
// low-level: req.login(id, user) / req.session().replace(map)
```

## Extension model

```rust
pub trait Plugin {
    fn id(&self) -> &'static str { /* type_name by default */ }
    fn requires(&self) -> &'static [&'static str] { &[] }
    fn meta(&self) -> PluginMeta { /* name + PLUGIN_SDK_VERSION */ }
    fn install(self, app: &mut App);
}

app.install(|app| { app.get("/x", handler); });
app.install(Cors::new());
```

Modules register themselves:

```rust
mod modules;
modules::register(&mut app);
```

### Writing plugins

Implement [`Plugin`](crates/ruvo-core/src/plugin.rs): set a short `id()`, fill `meta()` (description + optional `sdk`), register middleware/state/routes in `install`. Compatibility is checked against `PLUGIN_SDK_VERSION` (major mismatch or plugin-newer → build error; core-newer → warning).

Scaffold:

```bash
cargo ruvo generate plugin hello
cargo ruvo generate mailer Welcome
cargo ruvo generate job ProcessOrder   # alias: worker
cargo ruvo generate migration add_status_to_posts
cargo ruvo generate migration create_tags --fields name:string
cargo ruvo generate seed DemoUsers
cargo ruvo generate resource post --fields name:string,body:text
cargo ruvo generate resource post --api # JSON REST (+ smoke test)
# `generate crud <name>` → alias of `resource --api`
```

| Generate | Writes |
|----------|--------|
| `module <name>` | `src/modules/{name}.rs` + register |
| `plugin <name>` | `plugins/ruvo-{name}/` |
| `model <name> --fields …` | entity + SeaORM migration |
| `migration <name> [--fields …]` | `src/migrations/m{stamp}_{name}.rs` (blank, or create table if `--fields`; table from `create_*`) |
| `seed <Name>` | `src/seeds/{snake}.rs` + composed `seeds::run` |
| `mailer <Name>` | `src/mailers/{snake}.rs` (`Mailable`) + `views/mail/{snake}.html` (+ layout stub) |
| `job` / `worker <Name>` | `src/jobs/{snake}.rs` + `jobs::install(tasks)` helper |
| `resource <name> [--fields] [--api]` | module + routes; web views/test by default; `--api` = JSON CRUD + test |
| `crud <name>` | same as `resource --api` |

Project run/build:

| Command | What |
|---------|------|
| `cargo ruvo dev -p <pkg>` | watch `.rs` / `.env*` / `ruvo.toml` + restart; Vite if `frontend/` detected. Unix: graceful overlap (`SO_REUSEPORT`) by default; `--no-graceful` / `--drain-timeout <secs>` |
| `cargo ruvo build -p <pkg>` | frontend build (if any) + `cargo build --release` |
| `cargo ruvo serve -p <pkg>` | run release binary (`RUVO_ENV=production`) |
| `cargo ruvo db migrate -p <pkg> [up] [N]` | apply pending migrations (wraps `cargo run -- migrate`) |
| `cargo ruvo db down -p <pkg> [N]` | roll back N migrations (default 1) |
| `cargo ruvo db status -p <pkg>` | human-readable applied/pending table |
| `cargo ruvo db seed -p <pkg>` | run app `seed` CLI (`Db::seed`) |

Optional `[frontend]` in `ruvo.toml` (`enabled = false` to force off). No config needed when there is no Vite.

### `ruvo.toml` (app + plugins)

Declarative settings live in `[server]`, `[mail]`, `[storage]`, `[meta]`,
`[observability]`, `[db]` / `[redis]` (`url`), `[schedule.<job>]`, … with profile
overlays `[development.*]` / `[production.*]` (active profile = `RUVO_ENV` /
`RUVO_PROFILE`). Non-empty `DATABASE_URL` / `REDIS_URL` override toml URLs.
`ruvo_env::load()` cascade: `.env.{dev|prod|test}` → `.env.{mode}` →
`.env.local` → `.env` (process env always wins). Presets `App::web()` /
`App::api()` auto-load cwd `ruvo.toml`. See [ARCHITECTURE.md](ARCHITECTURE.md)
and `examples/cabinet/ruvo.toml`.

List installed plugins at runtime: `cargo run -- plugins`.

## Features (plugins)

Enable crates from the workspace:

| Feature | Crate |
|---------|--------|
| `web` | preset: cors, session, csrf, static, templates, meta, shield, cli, env, listen-reuseport |
| `api` | preset: cors, session, openapi, vld, cli, env, listen-reuseport |
| `cors` | `ruvo-cors` (`origins`, `exposed`, Vary) |
| `csrf` | `ruvo-csrf` (session double-submit; pulled by `web`) |
| `cookies` | `ruvo-cookies` |
| `session` | `ruvo-session` (`flash` / `take` / `flash_status` / `flash_old`; `Redirect::back`) |
| `compress` | `ruvo-compress` (`Compress::new()`, gzip/deflate/br) |
| `rate-limit` | `ruvo-rate-limit` (`RateLimitKey::Identity`, `RateLimit::login()`, headers, `key_fn`, `skip`) |
| `static-files` (default) | `ruvo-static` (`max_age`, `immutable`, `dotfiles_allow`) |
| `shield` | `ruvo-shield` (helmet-style headers; optional `csp`) |
| `templates` | `ruvo-templates` |
| `multipart` | `ruvo-core` form/files (`Request::input`, `Upload`, `UploadRules`) |
| `cli` | `ruvo-cli` |
| `vld` | `ruvo-vld` + `vld` |
| `openapi` | `ruvo-openapi` |
| `vld-openapi` | vld + openapi sugar |
| `i18n` | `ruvo-i18n` |
| `http-client` | `ruvo-http` |
| `mail` | `ruvo-mail` (SMTP via lettre; `Mail::try_from_env` / `fake`; bad URL → Err, not silent fake) |
| `mail-templates` | MiniJinja mail bodies: `.view(...)` / `Mailable` + `Content::view` (`{% extends %}` layouts) |
| `mail-markdown` | `.markdown(...)` / `Content::markdown` / `.markdown_view` (pulldown-cmark → HTML) |
| `storage` / `storage-s3` / `storage-gcs` / `storage-azure` / `storage-memory` | Object storage (`Storage::from_env`); uploads: `UploadRules` → `store`/`store_as` → `StoredFile` (cabinet avatar; MinIO: `examples/misc/storage`) |
| `passport` / `passport-session` / `passport-jwt` / `passport-oauth` | `ruvo-passport` (Passport, JWT + PAT `rvpat_…`, OAuth drivers) |
| `auth` / `auth-vld` / `auth-activity` | Fortify (`ruvo-auth`); `auth-activity` logs mutations via `ruvo-activity` |
| `activity` | Audit / activity log (`ruvo-activity`, table `activity_log`) |
| `notifications` / `notifications-ws` / `notifications-mail` / `notifications-auth` / `notifications-templates` | Inbox (`ruvo-notifications`, channels + ACL) |
| `meta` / `meta-templates` / `meta-i18n` / `meta-store` | SEO (`ruvo-meta`) |
| `ws` | `ruvo-ws` |
| `store` / `store-file` / `store-sql` / `store-redis` | KV (`ruvo::store::{Memory,File,Sql,Redis}` + `Cache`) |
| `tasks` / `tasks-file` / `tasks-sql` / `tasks-redis` | queue + scheduler + priorities (`Job` / `Dispatch` / `priority::*`); CLI `tasks list\|run\|schedule`; Console IO on `tasks run`; `[schedule.<job>]` in `ruvo.toml` |
| `db` / `db-sqlite` / `db-mysql` | SeaORM (`ruvo-db`; default postgres); `Page` / `paginate_ruvo` / `req.page_params()`; URL: `DATABASE_URL` or `[db] url` |
| `redis` | Shared Redis/Valkey pool (`Redis::from_env`, pub/sub, list queues); URL: `REDIS_URL` or `[redis] url` |
| `observability` / `observability-otel` / `observability-elasticsearch` | Prometheus `/metrics`; OTLP; ship tracing logs to Elasticsearch |
| `udp` / `quic-udp` / `sse-feed` / `env` / `tls` | networking / TLS |

```bash
cargo add ruvo --features web
# or
cargo add ruvo --features cors,session
```

## Observability

- **Request id (core):** `request_id()` middleware — inbound `x-request-id` or generated; echo on response; `RequestId` on `req`.
- **Logger:** structured `method` / `path` / `status` / `latency_ms` / `request_id`.
- **Prometheus:** feature `observability` → `app.install(Observability::new())` → `GET /metrics` (`http_requests_total`, `http_request_duration_seconds`, `http_requests_in_flight`; labels `method`, `status`, `route`).
- **OpenTelemetry:** feature `observability-otel` → `Observability::new().with_otel()`; set `OTEL_EXPORTER_OTLP_ENDPOINT` (+ optional `OTEL_SERVICE_NAME`). Prefer installing before `init_tracing` / `listen` so the OTLP layer can attach.
- **Elasticsearch logs:** feature `observability-elasticsearch` → `Observability::new().with_elasticsearch()`; ships tracing events via `_bulk` to `ELASTICSEARCH_URL` (index `ELASTICSEARCH_INDEX`, default `ruvo-logs`). Auth: `ELASTICSEARCH_API_KEY` or `ELASTICSEARCH_USERNAME`/`ELASTICSEARCH_PASSWORD`. Works with the normal `LogConfig` subscriber (hook layer).

## Logging

`listen` / `run` / `serve` install a default `tracing` subscriber (`LogConfig::from_env`, `try_init`).

| Control | Default | Notes |
|---------|---------|--------|
| `RUST_LOG` | `ruvo=info` | EnvFilter |
| `RUVO_LOG=off` | — | skip install |
| `RUVO_LOG_STDOUT` | `1` | `0`/`false` disables stdout |
| `RUVO_LOG_FILE` | unset | path → also log to file |
| `RUVO_LOG_ROTATE` | `size` | `size` \| `daily` \| `never` |
| `RUVO_LOG_ROTATE_SIZE` | `10MB` | for `size` (`parse_bytes`) |
| `RUVO_LOG_ROTATE_KEEP` | `5` | archived files to keep |
| `ELASTICSEARCH_URL` | unset | with `observability-elasticsearch` + `with_elasticsearch()` → bulk log sink |
| `ELASTICSEARCH_INDEX` | `ruvo-logs` | target index |

Stdout and file can be enabled together (two layers). File writes go through a non-blocking worker.

With `cli`, same options as flags: `--log-level`, `--log-file`, `--log-stdout false`, `--log-rotate`, `--log-rotate-size`, `--log-rotate-keep`. Or build [`LogConfig`](crates/ruvo-core/src/tracing_init.rs) in code and call `.install()`.

## Layout

- `crates/ruvo` — facade (`prelude`, `store`, `tasks`, `AppError`)
- `crates/ruvo-core` — `App`, router, request/response, server, `Cell`/`Slot`
- `crates/cargo-ruvo` — `cargo ruvo new` / `generate` / `dev` / `build` / `serve` / `db`
- `plugins/*` — optional crates behind features
- `examples/*` — runnable demos (`cargo run -p hello`); see [examples/README.md](examples/README.md)

### Tests & coverage

Library packages (`plugins/*`, `ruvo-core`, `ruvo`, `ruvo-testing`) must stay at **≥80% line coverage**.

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
./scripts/coverage.sh          # fail-under-lines=80 → target/llvm-cov/lcov.info
```

Examples and `cargo-ruvo` are excluded from the gate. CI: `.github/workflows/coverage.yml`.

### Share (`Cell` / `Slot`)

Cross-task handles via `app.state` (no worker manager):

- `Cell<T: Clone>` — counters / flags (`get` / `set` / `update` / `changed`)
- `Slot<T>` — ownership handoff for sockets/streams (`put` / `take`; unread `put` replaces)

See `examples/misc/share_demo`.

## Bind / listen

- `app.listen(3000).await` — common case (CLI + serve on `0.0.0.0:port`)
- `app.bind("127.0.0.1:8080").serve().await` — custom address (`Bind` lives in `ruvo::extend`)
- `app.bind("[::]:8443").tls(t)?.http(Http::all()).serve().await` — TLS / HTTP modes
- `Bind::Port(n)` binds IPv4 `0.0.0.0`; dual-stack via `"[::]:3000"` where the OS allows it

## Custom error pages / panic

```rust
app.not_found(|| async { Html("nope").into_response().status(404) });
app.error_handler(|err| async move { /* … */ });
```

Handler panics become **500** (`CatchUnwind`); prefer `Result` / typed errors over `panic!`.

## Stability / versions

Pre-1.0: breaking changes without a major bump. `ruvo` **0.4** tracks `ruvo-core` **0.4**.
