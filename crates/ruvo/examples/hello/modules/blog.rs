use ruvo::{Error, Request, Response, Router};

pub fn routes() -> Router {
    let mut r = Router::new();
    r.get("/", list_posts);
    r.get("/:id", show_post);

    let mut admin = Router::new();
    admin.use_middleware(admin_only);
    admin.get("/", dashboard);
    r.mount("/admin", admin);

    r
}

async fn list_posts(_req: Request) -> Response {
    Response::json(&serde_json::json!({
        "posts": [
            { "id": 1, "title": "Hello Ruvo" },
            { "id": 2, "title": "Express vibes" }
        ]
    }))
}

async fn show_post(req: Request) -> Response {
    let id = req.param("id").unwrap_or("?");
    Response::json(&serde_json::json!({
        "id": id,
        "title": format!("Post {id}")
    }))
}

async fn admin_only(mut req: Request, next: ruvo::Next) -> Response {
    match req.header("x-admin") {
        Some("1") => {
            req.set(AdminUser {
                name: "admin".into(),
            });
            next(req).await
        }
        _ => Error::Unauthorized.into_response(),
    }
}

struct AdminUser {
    name: String,
}

async fn dashboard(req: Request) -> Response {
    let name = req
        .get::<AdminUser>()
        .map(|u| u.name.as_str())
        .unwrap_or("?");
    Response::text(format!("admin dashboard ({name})"))
}
