# Ruvo

Express-like HTTP for Rust: `App`, `Router`, middleware, plugins — Hyper stays hidden.

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

Fifteen lines, no `unwrap`, no `map_err`, no bind enums in app code. Use `ruvo::{Cors, Static, …}` for plugins; everyday names come from [`prelude`](crates/ruvo/src/lib.rs).

## Extension model

```rust
pub trait Plugin {
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

## Features (plugins)

Enable crates from the workspace:

| Feature | Crate |
|---------|--------|
| `static-files` (default) | `ruvo-static` |
| `cors` | `ruvo-cors` |
| `cookies` | `ruvo-cookies` |
| `session` | `ruvo-session` (+ cookies) |
| `compress` | `ruvo-compress` |
| `rate-limit` | `ruvo-rate-limit` |
| `templates` | `ruvo-templates` |
| `multipart` | `ruvo-multipart` |
| `cli` | `ruvo-cli` |
| `vld` | `ruvo-vld` + `vld` |
| `openapi` | `ruvo-openapi` |
| `vld-openapi` | vld + openapi sugar |
| `i18n` | `ruvo-i18n` |
| `ws` | `ruvo-ws` |
| `store` / `store-file` / `store-postgres` / `store-sqlite` | KV (`ruvo::store::{Memory,File,…}`) |
| `tasks` / `tasks-file` / `tasks-postgres` / `tasks-sqlite` | queue (`ruvo::tasks::{Memory,File,…}`) |
| `db` | SeaORM Postgres (`ruvo-db`) |
| `udp` / `quic-udp` / `sse-feed` / `env` / `tls` | networking / TLS |

```bash
cargo add ruvo --features cors,session
```

## Logging

`listen` / `run` / `serve` install a default `tracing` subscriber via `try_init` (`RUST_LOG`, default `ruvo=info`). Set `RUVO_LOG=off` to skip. With `cli`, `ServerArgs::init_tracing()` still applies `--log-level`.

## Layout

- `crates/ruvo` — facade (`prelude`, `store`, `tasks`, `AppError`)
- `crates/ruvo-core` — `App`, router, request/response, server
- `crates/cargo-ruvo` — `cargo ruvo new` / `generate`
- `plugins/*` — optional crates behind features

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
