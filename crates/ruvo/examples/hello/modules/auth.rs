use ruvo::{Html, Json, Request, Router};
use serde::Deserialize;

pub fn routes() -> Router {
    let mut r = Router::new();
    r.get("/login", login_form);
    r.post("/login", login_submit);
    r
}

async fn login_form(_req: Request) -> Html<&'static str> {
    Html(include_str!("../views/login.html"))
}

#[derive(Deserialize)]
struct LoginBody {
    user: Option<String>,
}

async fn login_submit(mut req: Request) -> Json<serde_json::Value> {
    let user = match req.content_type() {
        Some(ct) if ct.contains("json") => req
            .json::<LoginBody>()
            .await
            .ok()
            .and_then(|b| b.user),
        _ => req
            .form::<LoginBody>()
            .await
            .ok()
            .and_then(|b| b.user),
    }
    .unwrap_or_else(|| "anonymous".into());

    Json(serde_json::json!({
        "message": format!("welcome, {user}"),
    }))
}
