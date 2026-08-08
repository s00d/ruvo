//! Axum stand: identical fixture bodies.

use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use stand_fixtures::{
    ABOUT, BLOG, CONTACT, CONTENT_TYPE_HTML, CONTENT_TYPE_JSON, HEALTH_JSON, HOME, POST_HELLO,
};

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9102);

    let app = Router::new()
        .route("/", get(|| async { html(HOME) }))
        .route("/about", get(|| async { html(ABOUT) }))
        .route("/blog", get(|| async { html(BLOG) }))
        .route(
            "/blog/{slug}",
            get(|Path(slug): Path<String>| async move {
                if slug == "hello" {
                    html(POST_HELLO)
                } else {
                    StatusCode::NOT_FOUND.into_response()
                }
            }),
        )
        .route("/contact", get(|| async { html(CONTACT) }))
        .route("/api/health", get(|| async { json_raw(HEALTH_JSON) }));

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("bind");
    eprintln!("stand_axum listening on http://127.0.0.1:{port}");
    axum::serve(listener, app).await.expect("serve");
}

fn html(body: &'static str) -> Response {
    (
        [(header::CONTENT_TYPE, CONTENT_TYPE_HTML)],
        body,
    )
        .into_response()
}

fn json_raw(body: &'static str) -> Response {
    (
        [(header::CONTENT_TYPE, CONTENT_TYPE_JSON)],
        body,
    )
        .into_response()
}
