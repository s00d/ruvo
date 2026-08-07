//! Full JWT auth: users + refresh via `JwtAuth` + `AuthMigrator`.
//!
//! ```bash
//! export DATABASE_URL=postgres://postgres@localhost/ruvo
//! export JWT_SECRET=dev-secret-change-me
//! cargo run -p api_jwt -- migrate
//! cargo run -p api_jwt
//!
//! curl -s -X POST http://127.0.0.1:3000/auth/register \
//!   -H 'content-type: application/json' \
//!   -d '{"email":"ada@example.com","password":"secret123"}'
//! curl -s http://127.0.0.1:3000/api/me -H "authorization: Bearer <access_token>"
//! ```

use ruvo::{
    App, AuthMigrator, Db, JwtAuth, JwtAuthExt, Json, Request, Response, Result, Router,
};

fn build_app() -> App {
    let mut app = App::new();
    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(JwtAuth::from_env().mount("/auth"));

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
    let app = build_app();
    tracing::info!("API http://127.0.0.1:3000  /auth/*  /api/me");
    app.run().await
}
