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
    app.use_middleware(logger());
    app.install(Cors::new().origin("*"));

    app.get("/", home);
    app.get("/health", || async { Json(serde_json::json!({ "ok": true })) });
    modules::register(&mut app);

    app.listen(3000).await
}
```

Use `ruvo::{Cors, Csrf, Static, Meta, Sitemap, Robots, …}` for plugins; everyday names come from [`prelude`](crates/ruvo/src/lib.rs).

## Request input

With feature `multipart` (urlencoded works without it for classic forms):

```rust
// urlencoded or multipart — one parse, cached on the request
let data = req.input().await?;
let title = data.get("title");
let file = data.file("avatar");
file.save_in("./uploads", "a.png").await?;

// text fields → struct (both encodings)
let body: CreatePost = req.form().await?;

// download helper (file + Content-Disposition: attachment)
Response::download("report.pdf").await
// status: Response::json(&x).status(201) or (201, Json(x))
```

`req.json()` is unchanged. There is no separate multipart crate.

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
```

List installed plugins at runtime: `cargo run -- plugins`.

## Features (plugins)

Enable crates from the workspace:

| Feature | Crate |
|---------|--------|
| `web` | preset: cors, session, csrf, static, templates, meta, shield, cli, env |
| `api` | preset: cors, session, openapi, vld, cli, env |
| `cors` | `ruvo-cors` (`origins`, `exposed`, Vary) |
| `csrf` | `ruvo-csrf` (session double-submit; pulled by `web`) |
| `cookies` | `ruvo-cookies` |
| `session` | `ruvo-session` (destroy/regenerate, rolling, `save_uninitialized`, `.hook`) |
| `compress` | `ruvo-compress` (`Compress::new()`, gzip/deflate/br) |
| `rate-limit` | `ruvo-rate-limit` (headers, `key_fn`, `skip`) |
| `static-files` (default) | `ruvo-static` (`max_age`, `immutable`, `dotfiles_allow`) |
| `shield` | `ruvo-shield` (helmet-style headers; optional `csp`) |
| `templates` | `ruvo-templates` |
| `multipart` | `ruvo-core` form/files (`Request::input`, `Upload`) |
| `cli` | `ruvo-cli` |
| `vld` | `ruvo-vld` + `vld` |
| `openapi` | `ruvo-openapi` |
| `vld-openapi` | vld + openapi sugar |
| `i18n` | `ruvo-i18n` |
| `http-client` | `ruvo-http` |
| `mail` | `ruvo-mail` (SMTP via lettre; `Mail::fake` / `from_env`) |
| `passport` / `passport-session` / `passport-jwt` / `passport-oauth` | `ruvo-passport` (Passport strategies, JWT, OAuth2) |
| `meta` / `meta-templates` / `meta-i18n` / `meta-store` | SEO (`ruvo-meta`) |
| `ws` | `ruvo-ws` |
| `store` / `store-file` / `store-postgres` / `store-sqlite` | KV (`ruvo::store::{Memory,File,…}`) |
| `tasks` / `tasks-file` / `tasks-postgres` / `tasks-sqlite` | queue (`ruvo::tasks::{Memory,File,…}`) |
| `db` | SeaORM Postgres (`ruvo-db`) |
| `udp` / `quic-udp` / `sse-feed` / `env` / `tls` | networking / TLS |

```bash
cargo add ruvo --features web
# or
cargo add ruvo --features cors,session
```

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

Stdout and file can be enabled together (two layers). File writes go through a non-blocking worker.

With `cli`, same options as flags: `--log-level`, `--log-file`, `--log-stdout false`, `--log-rotate`, `--log-rotate-size`, `--log-rotate-keep`. Or build [`LogConfig`](crates/ruvo-core/src/tracing_init.rs) in code and call `.install()`.

## Layout

- `crates/ruvo` — facade (`prelude`, `store`, `tasks`, `AppError`)
- `crates/ruvo-core` — `App`, router, request/response, server, `Cell`/`Slot`
- `crates/cargo-ruvo` — `cargo ruvo new` / `generate`
- `plugins/*` — optional crates behind features
- `examples/*` — runnable demos (`cargo run -p hello`); see [examples/README.md](examples/README.md)

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
