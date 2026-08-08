---
title: passport
editLink: false
---

# `passport`

**Users + access/refresh JWT + personal access tokens** · crate `sova-passport` · id `passport`

```bash
cargo add sova --features passport,passport-jwt,passport-oauth,passport-session
```

| Feature | What you get |
|---------|-------------|
| `passport` | Auth strategies registry (`sova-passport`). |
| `passport-jwt` | JWT access + refresh + PAT. |
| `passport-oauth` | OAuth2 drivers (GitHub/Google/Apple/Custom). |
| `passport-session` | Session serialize/login for Passport. |

Passport-style authentication for Sova.

 - [`Passport`] — strategy registry, `authenticate`, session serialize/deserialize, login/logout
 - [`Auth`] / [`AuthMw`] — extract + verify strategies (Bearer, API key, JWT)
 - feature `jwt`: [`JwtAuth`] (users, refresh, migrate)
 - feature `oauth`: [`Oauth`] (OAuth2 code + PKCE)

## Usage

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
