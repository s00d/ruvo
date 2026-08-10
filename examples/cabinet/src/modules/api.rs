use crate::db;
use serde_json::json;
use sova::vld;
use sova::{
    doc_schema, CurrentUser, DbExt, Doc, DocVldExt, IntoResponse, Json, Meta, OpenApiDocExt,
    Request, Response, Result, ValidExt, ValidateRouteExt,
};

vld::schema! {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct ApiNote {
        pub id: i64 => vld::number().int(),
        pub title: String => vld::string().min(1),
        pub body: String => vld::string(),
    }
}

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct CreateApiNote {
        pub title: String => vld::string().min(1).max(120),
        pub body: String => vld::string().max(4000),
    }
}

doc_schema!(ApiNote, CreateApiNote);

pub fn register(app: &mut sova::App) {
    app.get("/api/me", api_me)
        .doc(Doc::new().ok_schema(json!({
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "email": { "type": "string" },
                "name": { "type": "string" }
            }
        })))
        .with(Meta::noindex());

    app.get("/api/notes", api_list_notes)
        .doc(Doc::new().ok_schema(json!({"type":"array"})))
        .with(Meta::noindex());

    app.post("/api/notes", api_create_note)
        .validate_body::<CreateApiNote>()
        .doc(Doc::new().body::<CreateApiNote>())
        .with(Meta::noindex());
}

async fn api_me(req: Request) -> Result<Response> {
    let Some(user) = req.get::<CurrentUser>() else {
        return Err(sova::Error::Unauthorized.into());
    };
    Ok(Json(json!({
        "id": user.id,
        "email": user.email,
        "name": user.name,
        "avatar_path": user.avatar_path,
        "email_verified": user.email_verified,
        "two_factor_enabled": user.two_factor_enabled,
        "roles": user.roles,
        "permissions": user.permissions,
    }))
    .into_response())
}

async fn api_list_notes(req: Request) -> Result<Response> {
    let Some(user) = req.get::<CurrentUser>() else {
        return Err(sova::Error::Unauthorized.into());
    };
    let db = req.db().clone();
    let notes = db::list_notes(&db, user.id, 100).await?;
    Ok(Json(notes).into_response())
}

async fn api_create_note(req: Request) -> Result<Response> {
    let Some(user) = req.get::<CurrentUser>() else {
        return Err(sova::Error::Unauthorized.into());
    };
    let body = req.valid::<CreateApiNote>().clone();
    let db = req.db().clone();
    let id = db::create_note(&db, user.id, &body.title, &body.body).await?;
    Ok((
        201,
        Json(json!({
            "id": id,
            "title": body.title,
            "body": body.body,
        })),
    )
        .into_response())
}
