//! Sova stand: same bodies as axum/actix (see `stand_fixtures`).
//! Minimal stack — no logger middleware (fair vs other frameworks).

use sova::{App, Html, IntoResponse, Request, Response, Result};
use stand_fixtures::{
    ABOUT, BLOG, CONTACT, CONTENT_TYPE_JSON, HEALTH_JSON, HOME, POST_HELLO,
};

#[tokio::main]
async fn main() -> Result<()> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9101);

    let mut app = App::new();
    app.get("/", || async { Html(HOME) });
    app.get("/about", || async { Html(ABOUT) });
    app.get("/blog", || async { Html(BLOG) });
    app.get("/blog/:slug", |req: Request| async move {
        match req.param("slug").as_deref() {
            Some("hello") => Html(POST_HELLO).into_response(),
            _ => Response::text("not found").status(404),
        }
    });
    app.get("/contact", || async { Html(CONTACT) });
    app.get("/api/health", || async {
        Response::bytes(HEALTH_JSON.as_bytes(), CONTENT_TYPE_JSON)
    });

    eprintln!("stand_sova listening on http://127.0.0.1:{port}");
    app.listen(port).await
}
