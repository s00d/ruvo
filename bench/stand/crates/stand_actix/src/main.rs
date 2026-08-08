//! Actix-web stand: identical fixture bodies.

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use stand_fixtures::{
    ABOUT, BLOG, CONTACT, CONTENT_TYPE_HTML, CONTENT_TYPE_JSON, HEALTH_JSON, HOME, POST_HELLO,
};

#[get("/")]
async fn home() -> impl Responder {
    html(HOME)
}

#[get("/about")]
async fn about() -> impl Responder {
    html(ABOUT)
}

#[get("/blog")]
async fn blog() -> impl Responder {
    html(BLOG)
}

#[get("/blog/{slug}")]
async fn post(slug: web::Path<String>) -> impl Responder {
    if slug.as_str() == "hello" {
        html(POST_HELLO)
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[get("/contact")]
async fn contact() -> impl Responder {
    html(CONTACT)
}

#[get("/api/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok()
        .content_type(CONTENT_TYPE_JSON)
        .body(HEALTH_JSON)
}

fn html(body: &'static str) -> HttpResponse {
    HttpResponse::Ok()
        .content_type(CONTENT_TYPE_HTML)
        .body(body)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9103);

    eprintln!("stand_actix listening on http://127.0.0.1:{port}");
    HttpServer::new(|| {
        App::new()
            .service(home)
            .service(about)
            .service(blog)
            .service(post)
            .service(contact)
            .service(health)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
