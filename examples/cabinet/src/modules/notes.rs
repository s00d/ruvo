use sova::vld;
use crate::db;
use sova::{
    doc_schema, CsrfExt, CurrentUser, DbExt, Meta, PageExt, Redirect, RenderExt, Request, Response,
    Result, Router, SessionExt, ValidExt, ValidateRouteExt,
};
use serde_json::json;

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct NoteForm {
        pub title: String => vld::string().min(1).max(120),
        pub body: String => vld::string().max(4000),
        pub csrf: String => vld::string().min(8),
    }
}

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct DeleteNoteForm {
        pub id: String => vld::string().min(1),
        pub csrf: String => vld::string().min(8),
    }
}

doc_schema!(NoteForm, DeleteNoteForm);

pub fn mount(r: &mut Router) {
    r.get("/notes", list).with(Meta::noindex());
    r.post("/notes", create).validate_form::<NoteForm>();
    r.post("/notes/delete", delete).validate_form::<DeleteNoteForm>();
}

async fn list(req: Request) -> Result<Response> {
    let user = req.get::<CurrentUser>().expect("CurrentUser").clone();
    let db = req.db().clone();
    let page = db::paginate_notes(&db, user.id, req.page_params()).await?;
    let csrf = req.csrf_token();
    Ok(req.render(
        "cabinet/notes.html",
        json!({
            "user": { "name": user.name },
            "notes": page.data,
            "page": page.page,
            "last_page": page.last_page,
            "total": page.total,
            "csrf": csrf,
        }),
    )?)
}

async fn create(req: Request) -> Result<Response> {
    let form = req.valid::<NoteForm>().clone();
    let user = req.get::<CurrentUser>().expect("CurrentUser");
    let db = req.db().clone();
    db::create_note(&db, user.id, &form.title, &form.body).await?;
    req.flash_status("Note created");
    Ok(Redirect::see_other("/cabinet/notes").into_response())
}

async fn delete(req: Request) -> Result<Response> {
    let form = req.valid::<DeleteNoteForm>().clone();
    let id: i64 = form
        .id
        .parse()
        .map_err(|_| sova::Error::BadRequest("bad note id".into()))?;
    let user = req.get::<CurrentUser>().expect("CurrentUser");
    let db = req.db().clone();
    db::delete_note(&db, user.id, id).await?;
    req.flash_status("Note deleted");
    Ok(Redirect::back_or(&req, "/cabinet/notes").into_response())
}

use sova::IntoResponse;
