Compose JWT/PAT (and OAuth) **onto `App::api()`** so you keep Cors, probes, and `/docs`.

```rust
use sova::prelude::*;
use sova::{
    AuthMigrator, Db, JwtAuth, JwtAuthExt, Json, Parser, Request, Router, ServerArgs,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();
    let _ = sova::sova_env::load();

    let mut app = App::api().title("JWT API").version("1.0").into_app();
    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(JwtAuth::from_env().mount("/auth"));

    let mut api = Router::new();
    api.use_middleware(JwtAuth::guard());
    api.get("/me", |req: Request| async move {
        Ok::<_, sova::Error>(Json(req.require_auth_user()?.clone()))
    });
    app.mount("/api", api);

    app.run().await
}
```

```bash
export DATABASE_URL=postgres://postgres@localhost/sova
export JWT_SECRET=dev-secret-change-me
cargo run -p api_jwt -- migrate
cargo run -p api_jwt
```

OAuth providers: `cargo run -p api_oauth`. API keys: `api_auth`. Session browser login: feature `passport-session` (often via Fortify).
