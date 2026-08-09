---
title: auth
editLink: false
---

# `auth`

**Register/login, verify, reset, 2FA, profile, roles**

| | |
|--|--|
| Crate | [`sova-auth`](https://docs.rs/sova-auth/0.1.6) `0.1.6` |
| Plugin id | `fortify` |
| Category | Auth |

## Install

```bash
cargo add sova --features auth
```

## Features

| Feature | What you get |
|---------|-------------|
| `auth` | Fortify — register/login/verify/reset/2FA/RBAC. |
| `auth-activity` | Fortify mutations write activity events. |
| `auth-mail` | Email verify/reset (needs `mail` + Fortify mail helpers). |
| `auth-vld` | Fortify forms wired to `vld` flash/form. |

## Overview

**When:** register / login / verify / reset / 2FA / profile / roles (Fortify-style).

**Does:**
- Web forms + JSON API mounts
- Feature flags: Registration, ResetPasswords, EmailVerification, TwoFactor, …
- `Fortify::guard()` for protected routers
- `req.login_user` / `req.logout_user`
- Optional mail + activity

### Example

```rust
app.install(Db::from_env().migrations::<AuthMigrator>());
app.install(Mail::from_env()); // only if Reset / Verify
app.install(
  Fortify::new()
    .features([AuthFeature::Registration, AuthFeature::ResetPasswords])
    .home("/cabinet"),
);
cabinet.use_middleware(Fortify::guard());
```

### Notes
- Needs **db + session**
- Add **mail** only for email-backed features (`auth-mail`)

### Config

```bash
FORTIFY_SECRET=…     # or APP_KEY — token signing
PUBLIC_URL=https://… # links in verify/reset mail
APP_NAME=MyApp
```

No `[auth]` TOML section — features/paths are builder (`Fortify::new().features([...]).home(...)`).

## Quick start

Fortify sits **on the web preset** (sessions, csrf, templates already there). Add Db + Fortify; add **Mail only** when enabling `EmailVerification` or `ResetPasswords`.

```rust
// Registration-only — no Mail
use sova::prelude::*;
use sova::{AuthFeature, AuthMigrator, Db, Fortify, Parser, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("News")
        .public_url("http://127.0.0.1:3000")
        .into_app();

    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(
        Fortify::new()
            .features([AuthFeature::Registration])
            .web_forms(true)
            .no_api()
            .home("/")
            .login_redirect("/login"),
    );

    app.run().await
}
```

```rust
// With reset — Mail before Fortify
use sova::prelude::*;
use sova::{
    AuthFeature, AuthMigrator, Db, Fortify, Mail, Parser, ServerArgs,
};

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Cabinet")
        .public_url("https://example.com")
        .into_app();

    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(Mail::from_env());
    app.install(
        Fortify::new()
            .features([
                AuthFeature::Registration,
                AuthFeature::ResetPasswords,
            ])
            .api_mount("/api/auth")
            .login_redirect("/login")
            .home("/cabinet"),
    );

    modules::register(&mut app);
    app.run().await
}
```

```rust
// modules/mod.rs
use sova::{App, Fortify, Html, Router};

pub fn register(app: &mut App) {
    let mut cabinet = Router::new();
    cabinet.use_middleware(Fortify::guard());
    cabinet.get("/", || async { Html("<h1>Cabinet</h1>") });
    app.mount("/cabinet", cabinet);
}
```

Programmatic login (seed / impersonation): `req.login_user(cu)` / `req.logout_user()`.

See `examples/web/hackernews` (Registration-only) and `examples/cabinet` (full stack).

## Examples

- `examples/cabinet`
- `examples/web/hackernews`
- `examples/api/api_auth`
- `examples/basic/auth`

## Related

[`passport`](/plugins/passport) · [`session`](/plugins/session) · [`db`](/plugins/db) · [`mail`](/plugins/mail) · [`activity`](/plugins/activity)
