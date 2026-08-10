//! Story detail, upvote, comments.

use crate::db;
use serde_json::json;
use sova::vld;
use sova::{
    CsrfExt, CurrentUser, DbExt, Error, Fortify, Meta, Redirect, RenderExt, Request, Response,
    Result, Router, ValidationExt,
};

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct CommentForm {
        pub body: String => vld::string().min(1).max(4000),
        pub csrf: String => vld::string().min(8),
    }
}

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct VoteForm {
        pub csrf: String => vld::string().min(8),
    }
}

pub fn register(app: &mut sova::App) {
    app.get("/item/:id", show).with(Meta::page().title("Item"));

    let mut r = Router::new();
    r.use_middleware(Fortify::guard());
    r.post("/:id/vote", upvote);
    r.post("/:id/comment", add_comment);
    app.mount("/item", r);
}

async fn show(req: Request) -> Result<Response> {
    let id = parse_id(&req)?;
    let Some(story) = db::get_story(req.db(), id).await? else {
        return Err(Error::NotFound.into());
    };
    let comments = db::list_comments(req.db(), id).await?;
    let user = req.get::<CurrentUser>().cloned();
    let csrf = req.csrf_token();
    Ok(req.render(
        "item.html",
        json!({
            "story": story,
            "comments": comments,
            "user": user.map(|u| json!({ "id": u.id, "name": u.name })),
            "csrf": csrf,
        }),
    )?)
}

async fn upvote(mut req: Request) -> Result<Response> {
    let id = parse_id(&req)?;
    let _form: VoteForm = req.validate_form().await?;
    let user = req.get::<CurrentUser>().cloned().expect("guarded");
    let _ = db::upvote(req.db(), user.id, id).await?;
    Ok(Redirect::see_other(format!("/item/{id}")).into_response())
}

async fn add_comment(mut req: Request) -> Result<Response> {
    let id = parse_id(&req)?;
    let form: CommentForm = req.validate_form().await?;
    let user = req.get::<CurrentUser>().cloned().expect("guarded");
    db::add_comment(req.db(), user.id, id, form.body.trim().to_string()).await?;
    Ok(Redirect::see_other(format!("/item/{id}")).into_response())
}

fn parse_id(req: &Request) -> Result<i64> {
    let raw = req.param("id").unwrap_or("");
    raw.parse()
        .map_err(|_| Error::BadRequest("invalid id".into()))
        .map_err(Into::into)
}

use sova::IntoResponse;
