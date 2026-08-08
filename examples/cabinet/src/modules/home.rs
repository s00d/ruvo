use sova::{CsrfExt, CurrentUser, Meta, RenderExt, Request, Result};

pub fn register(app: &mut sova::App) {
    app.get("/", home).with(
        Meta::page()
            .title("Cabinet")
            .description("Sova kitchen-sink demo with Fortify auth"),
    );
}

async fn home(req: Request) -> Result<sova::Response> {
    let user = req.get::<CurrentUser>().cloned();
    let csrf = req.csrf_token();
    Ok(req.render(
        "home.html",
        serde_json::json!({
            "user": user.as_ref().map(|u| serde_json::json!({
                "name": u.name,
                "email": u.email,
                "email_verified": u.email_verified,
                "two_factor_enabled": u.two_factor_enabled,
            })),
            "csrf": csrf,
        }),
    )?)
}
