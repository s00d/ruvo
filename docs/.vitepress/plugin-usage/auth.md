Fortify sits **on the web preset** (sessions, csrf, templates already there). Add Db + Mail + Fortify, guard private mounts in modules.

```rust
// main.rs
use ruvo::prelude::*;
use ruvo::{
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
use ruvo::{App, Fortify, Html, Router};

pub fn register(app: &mut App) {
    let mut cabinet = Router::new();
    cabinet.use_middleware(Fortify::guard());
    cabinet.get("/", || async { Html("<h1>Cabinet</h1>") });
    app.mount("/cabinet", cabinet);
}
```

Programmatic login (seed / impersonation): `req.login_user(cu)` / `req.logout_user()`.

Full product reference: `cargo run -p cabinet`. JWT-only APIs → [passport](/plugins/passport).
