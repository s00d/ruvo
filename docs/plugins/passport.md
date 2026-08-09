---
title: passport
editLink: false
---

# `passport`

**Users + access/refresh JWT + personal access tokens**

| | |
|--|--|
| Crate | [`sova-passport`](https://docs.rs/sova-passport/0.1.2) `0.1.2` |
| Plugin id | `passport` |
| Category | Auth |

## Install

```bash
cargo add sova --features passport
```

## Features

| Feature | What you get |
|---------|-------------|
| `passport` | Auth strategies registry (JWT / PAT / OAuth). |
| `passport-jwt` | JWT access + refresh + personal access tokens. |
| `passport-oauth` | OAuth2 drivers (GitHub/Google/Apple/Custom). |
| `passport-session` | Session serialize/login for Passport. |

## Overview

**When:** JWT access/refresh, personal access tokens, OAuth login.

**Does:**
- Users + refresh tokens + PAT (`svpat_…`)
- `JwtAuth::guard` (Bearer JWT or PAT)
- OAuth: GitHub / Google / Apple / Custom

### Example

```rust
app.install(Passport::new().jwt(/* … */));
api.use_middleware(JwtAuth::guard());
```

### Notes
- OAuth env: `{NAME}_CLIENT_ID` / `{NAME}_CLIENT_SECRET`

## Quick start

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

## Examples

- `examples/api/api_jwt`
- `examples/api/api_oauth`

## Related

[`auth`](/plugins/auth) · [`session`](/plugins/session) · [`db`](/plugins/db)
