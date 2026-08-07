use ruvo::{
    CsrfExt, CurrentUser, DbExt, Meta, RenderExt, Request, Response, Result, Router,
};
use serde_json::json;

pub fn mount(r: &mut Router) {
    r.get("/", dashboard).with(
        Meta::page()
            .title("Dashboard")
            .description("Your cabinet home")
            .noindex(),
    );
    r.get("/profile", profile_get).with(Meta::noindex());
    r.get("/live", live).with(Meta::noindex());
}

async fn dashboard(req: Request) -> Result<Response> {
    let user = req.get::<CurrentUser>().expect("CurrentUser").clone();
    let db = req.db().clone();
    let notes = crate::db::list_notes(&db, user.id, 5).await?;
    let csrf = req.csrf_token();
    Ok(req.render(
        "cabinet/dashboard.html",
        json!({
            "user": {
                "id": user.id,
                "name": user.name,
                "email": user.email,
                "avatar_path": user.avatar_path,
                "email_verified": user.email_verified,
                "two_factor_enabled": user.two_factor_enabled,
                "roles": user.roles,
            },
            "notes": notes,
            "csrf": csrf,
            "verified": req.query("verified").is_some(),
        }),
    )?)
}

async fn profile_get(req: Request) -> Result<Response> {
    let user = req.get::<CurrentUser>().expect("CurrentUser").clone();
    let csrf = req.csrf_token();
    Ok(req.render(
        "cabinet/profile.html",
        json!({
            "user": {
                "name": user.name,
                "email": user.email,
                "avatar_path": user.avatar_path,
                "email_verified": user.email_verified,
                "two_factor_enabled": user.two_factor_enabled,
                "roles": user.roles,
            },
            "csrf": csrf,
            "password_ok": req.query("password").is_some(),
            "can_manage_users": user.has_permission("users.manage"),
        }),
    )?)
}

async fn live(req: Request) -> Result<Response> {
    let user = req.get::<CurrentUser>().expect("CurrentUser").clone();
    let csrf = req.csrf_token();
    Ok(req.render(
        "cabinet/live.html",
        json!({
            "user": { "name": user.name },
            "csrf": csrf,
        }),
    )?)
}
