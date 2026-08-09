---
title: auth
editLink: false
---

# `auth`

**Register/login, verify, reset, 2FA, profile, roles** · crate `sova-auth` `0.1.5` · id `fortify`

```bash
cargo add sova --features auth,auth-activity,auth-mail,auth-vld
```

| Feature | What you get |
|---------|-------------|
| `auth` | Fortify (register/login/verify/reset/2FA/RBAC). |
| `auth-activity` | Fortify mutations write activity events. |
| `auth-mail` | — |
| `auth-vld` | Fortify forms wired to `vld` flash/form. |

Fortify-style authentication for Sova (register, verify, reset, 2FA, RBAC).

 Builds on [`sova_passport`] (session login) + [`sova_db`]. Enable feature `mail`
 (and install [`sova_mail::Mail`]) for email verification / password reset.

```rust
 use sova_auth::{AuthMigrator, Feature, Fortify};
 // Facade re-exports the same enum as `AuthFeature`.

 app.install(Db::from_env().migrations::<AuthMigrator>());
 app.install(memory_sessions());
 app.install(
   Fortify::new()
     .features([Feature::Registration, Feature::ResetPasswords, /* … */])
     .api_mount("/api/auth")
     .login_redirect("/login")
     .home("/cabinet"),
 );
 // Mail only when ResetPasswords / EmailVerification:
 // app.install(Mail::from_env());
 cabinet.use_middleware(Fortify::guard());

 // Programmatic login (impersonation / seed / admin switch):
 let cu = load_current_user(db, id).await?.unwrap();
 req.login_user(cu);   // regenerates session + passport:user
 req.logout_user();
 ```

## Usage

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
