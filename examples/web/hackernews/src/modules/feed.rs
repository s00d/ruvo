//! Home / newest feeds. Logout is Fortify's `POST /logout`.

use crate::db;
use serde_json::json;
use sova::{CsrfExt, CurrentUser, DbExt, Meta, RenderExt, Request, Response, Result};

pub fn register(app: &mut sova::App) {
    app.get("/", index).with(Meta::page().title("Top"));
    app.get("/newest", newest)
        .with(Meta::page().title("Newest"));
}

async fn index(req: Request) -> Result<Response> {
    render_feed(req, false).await
}

async fn newest(req: Request) -> Result<Response> {
    render_feed(req, true).await
}

async fn render_feed(req: Request, newest: bool) -> Result<Response> {
    let stories = db::list_stories(req.db(), newest, 50).await?;
    let user = req.get::<CurrentUser>().cloned();
    let csrf = req.csrf_token();
    Ok(req.render(
        if newest { "newest.html" } else { "home.html" },
        json!({
            "stories": stories,
            "user": user.map(|u| json!({ "id": u.id, "name": u.name })),
            "csrf": csrf,
            "newest": newest,
        }),
    )?)
}
