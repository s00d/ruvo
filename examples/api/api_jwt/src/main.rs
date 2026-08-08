//! Full JWT auth + personal access tokens via `JwtAuth` + `AuthMigrator`.
//!
//! ```bash
//! export DATABASE_URL=postgres://postgres@localhost/sova
//! export JWT_SECRET=dev-secret-change-me
//! cargo run -p api_jwt -- migrate
//! # or: cargo sovax db migrate -p api_jwt
//! cargo run -p api_jwt
//!
//! # register / login → JWT
//! curl -s -X POST http://127.0.0.1:3000/auth/register \
//!   -H 'content-type: application/json' \
//!   -d '{"email":"ada@example.com","password":"secret123"}'
//! curl -s http://127.0.0.1:3000/api/me -H "authorization: Bearer <access_token>"
//!
//! # create PAT (machine client) — plaintext returned once
//! curl -s -X POST http://127.0.0.1:3000/auth/tokens \
//!   -H "authorization: Bearer <access_token>" \
//!   -H 'content-type: application/json' \
//!   -d '{"name":"ci","abilities":[]}'
//! curl -s http://127.0.0.1:3000/api/me -H "authorization: Bearer <svpat_…>"
//! ```

use sova::{
    App, AuthMigrator, Db, JwtAuth, JwtAuthExt, Json, Request, Result, Router,
};

fn build_app() -> App {
    let mut app = App::new();
    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(JwtAuth::from_env().mount("/auth"));
    app.with_probes();

    let mut api = Router::new();
    api.use_middleware(JwtAuth::guard());
    api.get("/me", |req: Request| async move {
        Ok::<_, sova::Error>(Json(req.require_auth_user()?.clone()))
    });
    app.mount("/api", api);

    app
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = sova::sova_env::load();
    let app = build_app();
    tracing::info!("API http://127.0.0.1:3000  /auth/*  /auth/tokens  /api/me");
    app.run().await
}
