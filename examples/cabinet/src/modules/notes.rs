use sova::vld;
use crate::db::{self, Note};
use sova::{
    doc_schema, Ability, AuthExt, CsrfExt, DbExt, Event, EventBus, Meta, PageExt, Policy, Redirect,
    RenderExt, Request, Response, Result, Router, SessionExt, ValidExt, ValidateRouteExt,
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

#[derive(Default)]
struct NotePolicy;

impl Policy<Note> for NotePolicy {
    fn view(&self, user: &sova::CurrentUser, r: &Note) -> bool {
        user.id == r.user_id || user.has_role("admin") || user.has_permission("notes.manage")
    }
    fn update(&self, user: &sova::CurrentUser, r: &Note) -> bool {
        self.view(user, r)
    }
    fn delete(&self, user: &sova::CurrentUser, r: &Note) -> bool {
        self.view(user, r)
    }
}

pub struct NoteCreated {
    pub note_id: i64,
    pub user_id: i64,
}

impl Event for NoteCreated {
    fn name(&self) -> &'static str {
        "note.created"
    }
}

pub fn mount(r: &mut Router) {
    r.get("/notes", list).with(Meta::noindex());
    r.post("/notes", create).validate_form::<NoteForm>();
    r.post("/notes/delete", delete).validate_form::<DeleteNoteForm>();
}

async fn list(req: Request) -> Result<Response> {
    let user = req.get::<sova::CurrentUser>().expect("CurrentUser").clone();
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
    let user = req.require_current_user()?;
    let db = req.db().clone();
    let note_id = db::create_note(&db, user.id, &form.title, &form.body).await?;
    if let Some(bus) = req.try_state::<EventBus>() {
        bus.dispatch(NoteCreated {
            note_id,
            user_id: user.id,
        });
    }
    req.flash_status("Note created");
    Ok(Redirect::see_other("/cabinet/notes").into_response())
}

async fn delete(req: Request) -> Result<Response> {
    let form = req.valid::<DeleteNoteForm>().clone();
    let id: i64 = form
        .id
        .parse()
        .map_err(|_| sova::Error::BadRequest("bad note id".into()))?;
    let db = req.db().clone();
    let note = db::find_note(&db, id)
        .await?
        .ok_or(sova::Error::NotFound)?;
    req.authorize::<NotePolicy, _>(Ability::Delete, &note)?;
    db::delete_note_by_id(&db, id).await?;
    req.flash_status("Note deleted");
    Ok(Redirect::back_or(&req, "/cabinet/notes").into_response())
}

use sova::IntoResponse;
