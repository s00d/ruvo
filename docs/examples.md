# Examples

![Examples](/banners/examples.svg?v=11)

Canonical shape: **`App::web()` / `App::api()` → modules → `app.run()`**.  
In-repo packages under `examples/` are runnable; some older demos still use `App::new()` — prefer the patterns here for new apps.

```bash
cargo run -p <package>
```

## API preset + validation + OpenAPI

What you write yourself: schema, route, `.doc(...)`. OpenAPI UI comes from the preset.

```rust
// main.rs
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
// modules/mod.rs
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

```bash
cargo run -p api_preset
# docs: http://127.0.0.1:3000/docs
```

Larger CRUD with the same ideas: `api_validated` (still documents every route; installs OpenAPI only because it does not use `App::api()`).

## Web preset + modules + Meta

```rust
// main.rs
use sova::prelude::*;
use sova::{Html, Meta, Parser, ServerArgs};

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Blog")
        .public_url("http://127.0.0.1:3000");

    app.get("/", || async { Html("<h1>Blog</h1>".to_string()) })
        .with(
            Meta::page()
                .title("Home")
                .description("Welcome"),
        );

    modules::register(&mut app);
    app.run().await
}
```

```rust
// modules/mod.rs
use sova::{App, Html, Router};

pub fn register(app: &mut App) {
    let mut posts = Router::new();
    posts.get("/", index).get("/:slug", show);
    app.mount("/posts", posts);
}

async fn index() -> Html<&'static str> {
    Html("<h1>Posts</h1>")
}

async fn show() -> Html<&'static str> {
    Html("<h1>Post</h1>")
}
```

Same layout as `cargo sovax new myapp --web`.

## Custom middleware

Onion on app or mount — timing header, auth gate, request-local state:

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

    app.use_middleware(named("timing", |req: Request, next: Next| async move {
        let start = Instant::now();
        let mut res = next(req).await;
        res = res.header(
            "x-response-time",
            format!("{}ms", start.elapsed().as_millis()),
        );
        res
    }));

    let mut admin = Router::new();
    admin.use_middleware(|req: Request, next: Next| async move {
        if req.header("x-admin") != Some("1") {
            return Response::text("forbidden").status(403);
        }
        next(req).await
    });
    admin.get("/", || async { Html("<h1>admin</h1>") });
    app.mount("/admin", admin);

    app.get("/", || async { Html("<h1>home</h1>") });
    app.run().await
}
```

See [Getting started → Custom middleware](/guide/getting-started#custom-middleware) and `examples/basic/hello` (`modules/blog.rs` admin gate).

## Templates (on the web preset)

If `views/` exists, `App::web()` already installs MiniJinja. Handlers only render:

```rust
use sova::{Request, Response, Result, RenderExt};
use serde::Serialize;

#[derive(Serialize)]
struct Page {
    title: String,
}

async fn home(req: Request) -> Response {
    req.render(
        "home.html",
        Page {
            title: "Sova".into(),
        },
    )
    .unwrap_or_else(|e| e.into_response())
}
```

Standalone demo (manual install for a minimal binary): `cargo run -p templates`.

## JWT API (Db + Passport on a composed app)

Auth plugins need a database. Preset still helps for Cors/docs if you want `App::api()`; JWT example composes explicitly:

```rust
use sova::{
    App, AuthMigrator, Db, JwtAuth, JwtAuthExt, Json, Request, Result, Router,
};

fn build_app() -> App {
    let mut app = App::api().title("JWT API").version("1.0").into_app();

    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(JwtAuth::from_env().mount("/auth"));

    let mut api = Router::new();
    api.use_middleware(JwtAuth::guard());
    api.get("/me", |req: Request| async move {
        Ok::<_, sova::Error>(Json(req.require_auth_user()?.clone()))
    });
    app.mount("/api", api);
    app
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = sova::sova_env::load();
    build_app().run().await
}
```

```bash
export DATABASE_URL=postgres://postgres@localhost/sova
export JWT_SECRET=dev-secret-change-me
cargo run -p api_jwt -- migrate
cargo run -p api_jwt
```

Related: `api_oauth`, `api_auth`.

## Tasks (scheduler + CLI console)

```rust
use sova::prelude::*;
use sova::{ask, bearer_guard, info, Job, Parser, ServerArgs, Tasks};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::new();
    let _ = app.configure_from_path("sova.toml");

    app.install(
        Tasks::new(Arc::new(sova::tasks::Memory::new()))
            .queues(["critical", "default", "mailer"])
            .scheduler_tick(Duration::from_secs(1))
            .job(
                Job::new("ping", |_task| async move {
                    tracing::info!("ping handled");
                    Ok(())
                })
                .every(Duration::from_secs(10)),
            )
            .job(Job::new("greet", |_task| async move {
                info("greet job");
                let name = ask("Your name").unwrap_or_else(|_| "world".into());
                info(&format!("hello, {name}"));
                Ok(())
            }))
            .exposed()
            .guard(bearer_guard("secret")),
    );

    app.get("/", || async {
        "CLI: tasks list | tasks schedule | tasks run greet"
    });

    app.run().await
}
```

```toml
# sova.toml — overrides `.every()` in code
[schedule.ping]
every = "15s"
```

```bash
cargo run -p tasks
cargo run -p tasks -- tasks list
cargo run -p tasks -- tasks run greet
```

## WebSocket room

```rust
use sova::prelude::*;
use sova::{Html, Message, Parser, ServerArgs, Ws, WsRouteExt};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web().site("Chat").public_url("http://127.0.0.1:3000");
    app.install(Ws::new());

    app.get("/", |_| async { Html(include_str!("../index.html")) });

    app.ws("/ws", |mut session| async move {
        let _room = session.join("chat");
        while let Some(Ok(msg)) = session.recv().await {
            if let Message::Text(text) = msg {
                session
                    .hub()
                    .broadcast("chat", Message::Text(text))
                    .await;
            }
        }
    });

    app.run().await
}
```

```bash
cargo run -p ws_chat
```

## Cabinet (full product stack)

Fortify, DB, mail, storage, tasks, notifications, OpenAPI, WS — modules + guarded `/cabinet`. Study `examples/cabinet` when presets alone are not enough.

```bash
cp examples/cabinet/.env.example examples/cabinet/.env
cargo sovax db migrate -p cabinet
cargo sovax db seed -p cabinet
cargo run -p cabinet
```

Seed: `demo@sova.local` / `demo1234`.

## Package map

| Area | Packages | Notes |
|------|----------|--------|
| API | `api_preset`, `api_validated`, `crud`, `api_jwt`, `api_oauth`, `api_auth` | Prefer `api_preset` as the template |
| Web | `templates`, `templates_i18n`, `upload`, `static_files`, `i18n`, `meta_blog` | Thin demos; new apps → `App::web()` |
| Basic | `hello`, `blog`, `auth`, `rest_api` | Older `App::new()` style |
| Realtime | `ws_chat`, `sse`, `sse_feed` | |
| Jobs | `tasks` | |
| Full app | `cabinet` | |
