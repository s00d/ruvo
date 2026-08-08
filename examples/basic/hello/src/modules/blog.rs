use sova::{Error, Json, Next, Request, Response, Router, Text};

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

async fn list_posts(_req: Request) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "posts": [
            { "id": 1, "title": "Hello Sova" },
            { "id": 2, "title": "Express vibes" }
        ]
    }))
}

async fn show_post(req: Request) -> Json<serde_json::Value> {
    let id = req.param("id").unwrap_or("?");
    Json(serde_json::json!({
        "id": id,
        "title": format!("Post {id}")
    }))
}

async fn admin_only(mut req: Request, next: Next) -> Response {
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

async fn dashboard(req: Request) -> Text<String> {
    let name = req
        .get::<AdminUser>()
        .map(|u| u.name.as_str())
        .unwrap_or("?");
    Text(format!("admin dashboard ({name})"))
}
