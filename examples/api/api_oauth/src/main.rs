//! OAuth2 login (GitHub) + JWT API guard.
//!
//! ```bash
//! export DATABASE_URL=postgres://postgres@localhost/ruvo
//! export JWT_SECRET=dev-secret
//! export GITHUB_CLIENT_ID=...
//! export GITHUB_CLIENT_SECRET=...
//! export OAUTH_PUBLIC_URL=http://127.0.0.1:3000
//! cargo run -p api_oauth -- migrate
//! cargo run -p api_oauth
//! # open http://127.0.0.1:3000/oauth/github
//! ```

use ruvo::{
    App, AuthMigrator, Db, JwtAuth, JwtAuthExt, Json, Oauth, OauthProvider, Request, Response,
    Result, Router,
};

fn build_app() -> App {
    let mut app = App::new();
    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(JwtAuth::from_env().mount("/auth"));
    app.install(
        Oauth::new()
            .provider(OauthProvider::github().from_env())
            .mount("/oauth"),
    );

    app.get("/health", |_r: Request| async { Response::text("ok") });

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
    tracing::info!("API http://127.0.0.1:3000  /oauth/github  /api/me");
    build_app().run().await
}
