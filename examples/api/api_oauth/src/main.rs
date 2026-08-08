//! OAuth2 login (GitHub / Google / optional Apple) + JWT API guard.
//!
//! ```bash
//! export DATABASE_URL=postgres://postgres@localhost/ruvo
//! export JWT_SECRET=dev-secret
//! export GITHUB_CLIENT_ID=...
//! export GITHUB_CLIENT_SECRET=...
//! export GOOGLE_CLIENT_ID=...
//! export GOOGLE_CLIENT_SECRET=...
//! # optional Apple Sign In:
//! # export APPLE_CLIENT_ID=...
//! # export APPLE_TEAM_ID=...
//! # export APPLE_KEY_ID=...
//! # export APPLE_PRIVATE_KEY="$(cat AuthKey.p8)"
//! export OAUTH_PUBLIC_URL=http://127.0.0.1:3000
//! cargo run -p api_oauth -- migrate
//! cargo run -p api_oauth
//! # open http://127.0.0.1:3000/oauth/github  or /oauth/google
//! ```

use ruvo::{
    App, Apple, AuthMigrator, Db, Driver, Github, Google, JwtAuth, JwtAuthExt, Json, Oauth,
    Request, Result, Router,
};

fn build_app() -> App {
    let mut app = App::new();
    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(JwtAuth::from_env().mount("/auth"));

    let mut oauth = Oauth::new()
        .provider(Github::new().from_env())
        .provider(Google::new().from_env())
        .mount("/oauth");
    if std::env::var("APPLE_CLIENT_ID").is_ok() {
        oauth = oauth.provider(Apple::new().from_env());
    }
    app.install(oauth);

    app.with_probes();

    let mut api = Router::new();
    api.use_middleware(JwtAuth::guard());
    api.get("/me", |req: Request| async move {
        Ok::<_, ruvo::Error>(Json(req.require_auth_user()?.clone()))
    });
    app.mount("/api", api);

    app
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = ruvo::ruvo_env::load();
    tracing::info!("API http://127.0.0.1:3000  /oauth/github|/google  /api/me");
    build_app().run().await
}
