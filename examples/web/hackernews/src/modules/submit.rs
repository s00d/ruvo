//! Submit a new story (auth required).

use crate::db;
use serde_json::json;
use sova::vld;
use sova::vld::schema::VldSchema;
use sova::{
    CsrfExt, CurrentUser, DbExt, Fortify, Meta, Redirect, RenderExt, Request, Response, Result,
    Router, ValidationExt,
};

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct SubmitForm {
        pub title: String => vld::string().min(3).max(300),
        pub url: Option<String> => vld::string().max(2000).optional(),
        pub text: Option<String> => vld::string().max(8000).optional(),
        pub csrf: String => vld::string().min(8),
    }
}

pub fn register(app: &mut sova::App) {
    let mut r = Router::new();
    r.use_middleware(Fortify::guard());
    r.get("/", form).with(Meta::page().title("Submit"));
    r.post("/", create);
    app.mount("/submit", r);
}

async fn form(req: Request) -> Result<Response> {
    let user = req.get::<CurrentUser>().cloned();
    let csrf = req.csrf_token();
    Ok(req.render(
        "submit.html",
        json!({
            "user": user.map(|u| json!({ "id": u.id, "name": u.name })),
            "csrf": csrf,
        }),
    )?)
}

async fn create(mut req: Request) -> Result<Response> {
    let form: SubmitForm = req.validate_form().await?;
    let user = req
        .get::<CurrentUser>()
        .cloned()
        .expect("Fortify::guard ensures CurrentUser");
    let url = form
        .url
        .filter(|u| !u.trim().is_empty())
        .map(|u| u.trim().to_string());
    let text = form
        .text
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.trim().to_string());
    if url.is_none() && text.is_none() {
        return Ok(req
            .render(
                "submit.html",
                json!({
                    "user": { "id": user.id, "name": user.name },
                    "csrf": req.csrf_token(),
                    "error": "Provide a URL and/or text.",
                    "title": form.title,
                }),
            )?
            .into_response());
    }
    let story =
        db::create_story(req.db(), user.id, form.title.trim().to_string(), url, text).await?;
    Ok(Redirect::see_other(format!("/item/{}", story.id)).into_response())
}

use sova::IntoResponse;
