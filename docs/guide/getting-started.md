# Getting started

![Getting started](/banners/getting-started.svg?v=7)

Sova’s idea is simple: **start from a preset**, put routes in **modules**, call **`app.run()`**.  
Do not hand-roll Cors + Session + logger + probes on every app — that is what `App::web()` / `App::api()` are for.

```bash
cargo add sova --features web
# or
cargo add sova --features api
```

Scaffold (same shape as the docs below):

```bash
cargo install --path crates/cargo-sovax   # from this repo
cargo sovax new blog --web
cargo sovax new ping-api --api
```

## Web app

`features = ["web"]`. Preset installs: `request_id`, `logger`, Cors, Shield, cookie sessions, Csrf, Static (`public/` → `/assets`), Templates (`views/`), Meta, Sitemap, Robots, health probes.

```rust
// src/main.rs
use sova::prelude::*;
use sova::{Html, Meta, Parser, ServerArgs};

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Blog")
        .public_url("https://example.com");

    app.get("/", home).with(
        Meta::page()
            .title("Home")
            .description("Welcome to the blog"),
    );

    modules::register(&mut app);
    app.run().await
}

async fn home() -> Html<&'static str> {
    Html("<h1>Blog</h1>")
}
```

```rust
// src/modules/mod.rs
use sova::{App, Html, Meta, Router};

pub fn register(app: &mut App) {
    let mut blog = Router::new();
    blog.get("/", list)
        .get("/:slug", show)
        .with(Meta::page().title("Posts"));
    app.mount("/blog", blog);
}

async fn list() -> Html<&'static str> {
    Html("<h1>Posts</h1>")
}

async fn show() -> Html<&'static str> {
    Html("<h1>Post</h1>")
}
```

Head tags are injected for HTML. Built-ins: `/sitemap.xml`, `/robots.txt`, `/healthz`, `/ready`.  
Prefer **`app.run().await`** (CLI + `HOST`/`PORT`) over bare `listen` in real apps.

## JSON API

`features = ["api"]`. Preset installs: `request_id`, `logger`, Cors, sessions, **OpenAPI + Scalar at `/docs`**, probes. You add **vld schemas**, **`.doc(...)`**, and handlers — not another `OpenApi::new(...)`.

```rust
// src/main.rs
use sova::prelude::*;
use sova::vld;
use sova::{
    doc_schema, Doc, DocVldExt, Json, OpenApiDocExt, Parser, Request, ServerArgs,
    ValidationError, ValidationExt,
};

mod modules;

vld::schema! {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Ping {
        pub message: String => vld::string().min(1).max(100),
    }
}

doc_schema!(Ping);

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::api().title("Ping API").version("1.0");
    modules::register(&mut app);
    app.run().await
}
```

```rust
// src/modules/mod.rs
use crate::Ping;
use sova::{
    App, Doc, DocVldExt, Json, OpenApiDocExt, Request, ValidationError, ValidationExt,
};

pub fn register(app: &mut App) {
    app.post("/ping", ping)
        .doc(Doc::new().body::<Ping>().ok::<Ping>());
}

async fn ping(mut req: Request) -> std::result::Result<Json<Ping>, ValidationError> {
    let body: Ping = req.validate().await?;
    Ok(Json(body))
}
```

Open Scalar at `http://127.0.0.1:3000/docs`. Same pattern in-tree: `cargo run -p api_preset`.

## Adding plugins on top of a preset

Presets already own the baseline. Extra plugins are **`app.install(...)` after** `App::web()` / `App::api()` — do not rebuild the stack with `App::new()`.

```rust
let mut app = App::web()
    .site("Cabinet")
    .public_url("https://example.com");

// Session/Csrf/Templates are already there.
app.install(Db::from_env().migrations::<AuthMigrator>());
app.install(Mail::from_env());
app.install(
    Fortify::new()
        .features([AuthFeature::Registration, AuthFeature::ResetPasswords])
        .home("/cabinet"),
);

modules::register(&mut app);
app.run().await
```

Need the underlying `App` (tests, custom listen): `let app = App::web().site("X").into_app();`.

## Custom middleware

This is the Express onion: `(Request, Next) -> Response`. Call `next(req).await` to continue, or return early. Scope it on the **app**, a **mounted router**, or a single route group.

### Timing + response header (app-wide)

```rust
use sova::prelude::*;
use sova::extend::named;
use sova::{Parser, ServerArgs};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Demo")
        .public_url("http://127.0.0.1:3000");

    // Preset already has request_id + logger; add your own layer:
    app.use_middleware(named("timing", |req: Request, next: Next| async move {
        let start = Instant::now();
        let mut res = next(req).await;
        let ms = start.elapsed().as_millis();
        res = res.header("x-response-time", format!("{ms}ms"));
        res
    }));

    app.get("/", || async { "ok" });
    app.run().await
}
```

### Auth gate on a mount (short-circuit)

```rust
// modules/admin.rs
use sova::{Error, Html, IntoResponse, Next, Request, Response, Router};

pub fn routes() -> Router {
    let mut admin = Router::new();
    admin.use_middleware(require_admin);
    admin.get("/", dashboard);
    admin
}

async fn require_admin(mut req: Request, next: Next) -> Response {
    match req.header("x-admin") {
        Some("1") => {
            req.set(AdminUser {
                name: "admin".into(),
            });
            next(req).await
        }
        _ => Error::Unauthorized.into_response(),
    }
}

struct AdminUser {
    name: String,
}

async fn dashboard(req: Request) -> Html<String> {
    let name = req
        .get::<AdminUser>()
        .map(|u| u.name.as_str())
        .unwrap_or("?");
    Html(format!("<h1>admin ({name})</h1>"))
}
```

```rust
// modules/mod.rs
pub fn register(app: &mut App) {
    app.mount("/admin", admin::routes());
}
```

Same pattern as `examples/basic/hello` (`x-admin: 1`) and cabinet (`Fortify::guard()` on `/cabinet`).

### Request-local work before the handler

```rust
cabinet.use_middleware(Fortify::guard());
cabinet.use_middleware(|mut req: Request, next: Next| async move {
    preload_unread(&mut req).await; // stash data on the request
    next(req).await
});
```

### Middleware with state (`with_state` / `with_leaked`)

Prefer one `Arc` (or process-lifetime leak) instead of cloning many captures into every future:

```rust
use sova::{with_state, Next, Request, Response};
use std::sync::atomic::{AtomicU64, Ordering};

struct Hits(AtomicU64);

app.use_middleware(with_state(Hits(AtomicU64::new(0)), |hits, req, next| async move {
    hits.0.fetch_add(1, Ordering::Relaxed);
    let mut res = next(req).await;
    res = res.header("x-hits", hits.0.load(Ordering::Relaxed).to_string());
    res
}));
```

Plugin authors often use `sova::extend::with_leaked` for immutable config (see [Plugin SDK](/api/plugin-sdk)). Label layers for `routes` / explain: `sova::extend::named("my-mw", …)`.

Order is onion: first `use_middleware` is outermost. Root stack runs **before** mount/route middleware.

## Forms, flash, uploads

With the **web** preset, sessions and CSRF are already installed:

```rust
async fn save(mut req: Request) -> Result<Redirect> {
    let form: NoteForm = req.validate_form().await?; // feature `vld` / `auth-vld`
    // …
    req.flash_status("Saved");
    Ok(Redirect::back(&req))
}
```

Uploads need `multipart` + `storage`:

```rust
async fn avatar(mut req: Request) -> Result<Redirect> {
    let data = req.input().await?;
    let file = data
        .file("avatar")
        .cloned()
        .ok_or_else(|| Error::bad_request("avatar required"))?;
    file.validate(
        &UploadRules::new()
            .max_bytes(2_000_000)
            .extensions(["png", "jpg", "webp"]),
    )?;
    let _stored = req.storage().store(&file, "avatars").await?;
    Ok(Redirect::see_other("/cabinet/profile"))
}
```

## When `App::new()` is appropriate

Use a bare app only when you are **not** shipping a web/API product stack (tiny demos, custom middleware order, net/UDP). Product apps should stay on presets. Kitchen-sink reference that still composes plugins deliberately: `examples/cabinet`.

## Testing

Dev-dependency `sova-testing`. Build with the same preset helpers you use in `main`:

```rust
fn app() -> App {
    let mut app = App::api().title("Ping").version("1.0").into_app();
    modules::register(&mut app);
    app
}
```

## Next

- [Concepts](./concepts) — request path and lifecycle  
- [Configuration](./configuration) — `sova.toml` / `.env`  
- [Plugins](/plugins/) — how each plugin extends the preset  
- [Examples](/examples) — full runnable patterns  
- [Performance](./performance) — Sova vs Axum vs Actix stand  
