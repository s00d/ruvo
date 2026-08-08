---
title: passport
editLink: false
---

# `passport`

**OAuth2 login (authorization code + PKCE)** · crate `ruvo-passport` · id `jwt-auth`

```bash
cargo add ruvo --features passport,passport-jwt,passport-oauth,passport-session
```

| Feature | What you get |
|---------|-------------|
| `passport` | Auth strategies registry (`ruvo-passport`). |
| `passport-jwt` | JWT access + refresh + PAT. |
| `passport-oauth` | OAuth2 drivers (GitHub/Google/Apple/Custom). |
| `passport-session` | Session serialize/login for Passport. |

Passport-style authentication for Ruvo.

 - [`Passport`] — strategy registry, `authenticate`, session serialize/deserialize, login/logout
 - [`Auth`] / [`AuthMw`] — extract + verify strategies (Bearer, API key, JWT)
 - feature `jwt`: [`JwtAuth`] (users, refresh, migrate)
 - feature `oauth`: [`Oauth`] (OAuth2 code + PKCE)

## Usage

Compose JWT/PAT (and OAuth) **onto `App::api()`** so you keep Cors, probes, and `/docs`.

```rust
use ruvo::prelude::*;
use ruvo::{
    AuthMigrator, Db, JwtAuth, JwtAuthExt, Json, Parser, Request, Router, ServerArgs,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();
    let _ = ruvo::ruvo_env::load();

    let mut app = App::api().title("JWT API").version("1.0").into_app();
    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(JwtAuth::from_env().mount("/auth"));

    let mut api = Router::new();
    api.use_middleware(JwtAuth::guard());
    api.get("/me", |req: Request| async move {
        Ok::<_, ruvo::Error>(Json(req.require_auth_user()?.clone()))
    });
    app.mount("/api", api);

    app.run().await
}
```

```bash
export DATABASE_URL=postgres://postgres@localhost/ruvo
export JWT_SECRET=dev-secret-change-me
cargo run -p api_jwt -- migrate
cargo run -p api_jwt
```

OAuth providers: `cargo run -p api_oauth`. API keys: `api_auth`. Session browser login: feature `passport-session` (often via Fortify).
