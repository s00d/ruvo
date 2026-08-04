# Ruvo

Express-like HTTP for Rust: `App`, `Router`, middleware, plugins — Hyper stays hidden.

```rust
use ruvo::{logger, App, Cors, Request, Response, Result, Router};

fn blog_routes() -> Router {
    let mut r = Router::new();
    r.get("/", list);
    r.get("/:slug", show);
    r
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.use_middleware(logger());
    app.install(Cors::new().origin("*"));
    app.mount("/blog", blog_routes());
    app.install(Static::new("/assets", "./public"));
    app.install(|app| {
        app.get("/health", |_| async { Response::json(&serde_json::json!({ "ok": true })) });
    });
    app.raw("/raw", |req| async move { /* hyper Response */ todo!() });
    app.listen(3000).await
}
```

Use `ruvo::{App, Cors, Static, ...}` as needed.

## Extension model

```rust
pub trait Plugin {
    fn install(self, app: &mut App);
}

app.install(|app| { app.get("/x", handler); });
app.install(Cors::new());
```

Modules return a `Router` and get mounted:

```rust
app.mount("/blog", blog::routes());
```

Middleware can stash typed values:

```rust
req.set(user);
let user = req.get::<User>();
```

Plugin / middleware state — use helpers instead of cloning fields into futures:

```rust
use ruvo::with_state;

app.use_middleware(with_state(cfg, |cfg, req, next| async move {
    // cfg: Arc<_> — one clone hidden inside with_state
    next(req).await
}));
```

Immutable process-lifetime config: `with_leaked(cfg, |cfg, req, next| …)` (`&'static`, no Arc).

Raw escape hatch (WebSocket/SSE/custom):

```rust
app.raw("/ws", |req: hyper::Request<Incoming>| async move { ... });
```

## Features

| Feature | What |
|---------|------|
| `static-files` (default) | `Static` plugin (`Response::file` / `file_in` are always core) |
| `cors` | `Cors` plugin |
| `cookies` | `CookieLayer` + `Cookies` + `ResponseCookieExt` (`.cookie()`) |
| `compress` | gzip/br response compression |
| `rate-limit` | in-memory sliding window by IP |
| `session` | `SessionStore` + `MemoryStore` / `NullStore` |
| `templates` | `TemplateEngine` + MiniJinja |
| `multipart` | `MultipartExt` (`multer`) for file uploads |
| `cli` | `ServerArgs` / `ListenArgs` (`clap`) for local `--host`/`--port`/`--log-level` |

## Examples

```bash
cargo run -p ruvo --example hello --all-features
cargo run -p ruvo --example rest-api
cargo run -p ruvo --example upload --features multipart
cargo run -p ruvo --example cli --features cli -- --port 3010 --log-level debug
```

Call `ruvo::init_tracing()` in `main` (or `ServerArgs::init_tracing()` with `cli`) so listening banners and `logger()` middleware are visible.

## Core ideas

- `App` is `DerefMut<Router>` — one routing API
- Routes compile once at `listen` (`matchit` + precomputed middleware chains)
- Response body: bytes or stream; `take_body` / `collect` / `set_body` for middleware
- Lifecycle: `on_startup` (fail → no listen) / `on_shutdown` after connection drain
- Introspection: `app.route_entries()`
- `req.state::<T>()` panics if missing; `try_state` for Option
- Handlers may return `Html` / `Json` (`IntoResponse`) instead of building `Response` by hand
- `not_found` / `error_handler` / `max_body_size` (default 2 MiB → 413)
- Graceful shutdown on Ctrl-C and SIGTERM (Unix); connection `JoinSet` drain (`drain_timeout`, default 20s)
- Listen: `listen(port)`, `listen_on(addr)`, `listen_str("127.0.0.1:3000")`, `listen_env(default_port)` (`PORT`/`HOST`), `listen_listener`, `listen_uds` (Unix), `*_with_shutdown(fut)` for tests / embedding
- `listen(port)` binds `0.0.0.0` (IPv4). For dual-stack use `listen_str("[::]:3000")` where the OS allows it
- Limits: `max_connections`, `request_timeout`, `header_read_timeout` / `idle_timeout` (keep-alive quiet wait), `max_headers`, `max_buf_size`, `keep_alive`

## TLS / HTTP2

Ruvo speaks HTTP/1.1 cleartext. Put nginx, Caddy, or Cloudflare in front for TLS and HTTP/2.

## Workspace

Virtual workspace: facade in `crates/ruvo/`, core in `crates/ruvo-core/`, plugins under `plugins/`. Depend on the `ruvo` facade; features select plugins. Not published separately yet.
