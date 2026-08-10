//! HTML auth pages (JSON API via axios + session cookie).

use serde_json::json;
use sova::{
    mark_email_verified, parse_verify_token, AuthExt, CsrfExt, DbExt, IntoResponse, Meta, Redirect,
    RenderExt, Request, Response, Result,
};

pub fn register(app: &mut sova::App) {
    app.get("/register", register_get).with(
        Meta::page()
            .title("Register")
            .description("Create a cabinet account"),
    );
    app.get("/login", login_get).with(
        Meta::page()
            .title("Login")
            .description("Sign in to your cabinet"),
    );
    app.get("/forgot-password", forgot_get)
        .with(Meta::noindex());
    app.get("/reset-password", reset_get).with(Meta::noindex());
    app.get("/email/verify", verify_get).with(Meta::noindex());
    app.get("/two-factor-challenge", challenge_get)
        .with(Meta::noindex());
    app.get("/user/confirm-password", confirm_password_get)
        .with(Meta::noindex());
    app.get("/user/two-factor-authentication", two_factor_get)
        .with(Meta::noindex());
    app.get("/admin/roles", admin_roles_get)
        .with(Meta::noindex());
}

async fn register_get(req: Request) -> Result<Response> {
    let csrf = req.csrf_token();
    Ok(req.render("auth/register.html", json!({ "csrf": csrf, "user": null }))?)
}

async fn login_get(req: Request) -> Result<Response> {
    let csrf = req.csrf_token();
    Ok(req.render("auth/login.html", json!({ "csrf": csrf, "user": null }))?)
}

async fn forgot_get(req: Request) -> Result<Response> {
    let csrf = req.csrf_token();
    Ok(req.render(
        "auth/forgot.html",
        json!({
            "csrf": csrf,
            "user": null,
            "sent": req.query("sent").is_some(),
        }),
    )?)
}

async fn reset_get(req: Request) -> Result<Response> {
    let csrf = req.csrf_token();
    Ok(req.render(
        "auth/reset.html",
        json!({
            "csrf": csrf,
            "user": null,
            "email": req.query("email").unwrap_or(""),
            "token": req.query("token").unwrap_or(""),
        }),
    )?)
}

async fn verify_get(req: Request) -> Result<Response> {
    if let Some(token) = req.query("token") {
        let secret = std::env::var("FORTIFY_SECRET")
            .or_else(|_| std::env::var("APP_KEY"))
            .unwrap_or_else(|_| "dev-fortify-secret-change-me".into());
        match parse_verify_token(&secret, token) {
            Ok(id) => {
                let db = req.db().clone();
                mark_email_verified(&db, id).await?;
                return Ok(Redirect::see_other("/cabinet?verified=1").into_response());
            }
            Err(e) => {
                return Ok(Redirect::see_other(format!(
                    "/email/verify?error={}",
                    urlencoding::encode(&e.to_string())
                ))
                .into_response());
            }
        }
    }
    let csrf = req.csrf_token();
    let user = req.get::<sova::CurrentUser>().cloned();
    Ok(req.render(
        "auth/verify.html",
        json!({
            "csrf": csrf,
            "user": user.as_ref().map(|u| json!({
                "name": u.name,
                "email": u.email,
                "email_verified": u.email_verified,
            })),
            "sent": req.query("sent").is_some(),
            "error": req.query("error"),
        }),
    )?)
}

async fn challenge_get(req: Request) -> Result<Response> {
    let csrf = req.csrf_token();
    Ok(req.render(
        "auth/two_factor_challenge.html",
        json!({
            "csrf": csrf,
            "user": null,
        }),
    )?)
}

async fn confirm_password_get(req: Request) -> Result<Response> {
    let user = req
        .get::<sova::CurrentUser>()
        .cloned()
        .ok_or(sova::Error::Unauthorized)?;
    let csrf = req.csrf_token();
    Ok(req.render(
        "auth/confirm_password.html",
        json!({
            "csrf": csrf,
            "user": { "name": user.name },
        }),
    )?)
}

async fn two_factor_get(req: Request) -> Result<Response> {
    let user = req
        .get::<sova::CurrentUser>()
        .cloned()
        .ok_or(sova::Error::Unauthorized)?;
    let csrf = req.csrf_token();
    Ok(req.render(
        "auth/two_factor.html",
        json!({
            "csrf": csrf,
            "user": {
                "name": user.name,
                "email": user.email,
                "two_factor_enabled": user.two_factor_enabled,
            },
            "secret": null,
            "otpauth_url": null,
        }),
    )?)
}

async fn admin_roles_get(req: Request) -> Result<Response> {
    let user = req.require_permission("users.manage")?.clone();
    let csrf = req.csrf_token();
    Ok(req.render(
        "auth/admin_roles.html",
        json!({
            "csrf": csrf,
            "user": {
                "name": user.name,
                "email": user.email,
                "roles": user.roles,
            },
        }),
    )?)
}
