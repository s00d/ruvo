//! Auth action handlers (shared by web redirects + JSON API).

use crate::feature::Feature;
use crate::guard::{self, AuthExt};
#[cfg(feature = "mail")]
use crate::mail::{send_reset, send_verify};
use crate::state::{
    read_creds, require_feature, wants_json, CredsForm, FortifyState, PENDING_2FA_KEY,
};
use crate::store::{self, CurrentUser};
use crate::token::{
    consume_recovery_code, generate_recovery_codes, hash_recovery_codes, make_verify_token,
    parse_verify_token,
};
use crate::two_factor;
use sova_core::{Error, IntoResponse, Json, Redirect, Request, Response, Result};
use sova_db::DbExt;
use sova_passport::verify_password;
use sova_session::SessionExt;
use serde_json::json;

#[cfg(feature = "activity")]
use crate::activity_log::log_event;

fn ok_redirect(path: &str) -> Response {
    Redirect::see_other(path).into_response()
}

fn err_redirect(path: &str, msg: &str) -> Response {
    let loc = format!("{path}?error={}", urlencoding::encode(msg));
    Redirect::see_other(loc).into_response()
}

fn json_ok(body: serde_json::Value) -> Response {
    Json(body).into_response()
}

fn json_err(status: u16, msg: &str) -> Response {
    Error::custom(status, Json(json!({ "error": msg }))).into_response()
}

fn json_field_err(status: u16, field: &str, msg: &str) -> Response {
    Error::custom(
        status,
        Json(json!({ "errors": { field: [msg] } })),
    )
    .into_response()
}

fn field_fail(req: &Request, path: &str, field: &str, msg: &str, status: u16) -> Response {
    if wants_json(req) {
        return json_field_err(status, field, msg);
    }
    #[cfg(feature = "vld")]
    {
        use sova_core::FormData;
        use serde_json::json;
        req.flash_errors(&json!({ field: msg }));
        let mut old = serde_json::Map::new();
        if let Some(data) = req.get::<FormData>() {
            for (k, values) in data.text_map() {
                if let Some(v) = values.first() {
                    // Never echo passwords into flash_old.
                    if k == "password" || k == "password_confirmation" || k == "current_password" {
                        continue;
                    }
                    old.insert(k.clone(), json!(v));
                }
            }
        }
        if !old.is_empty() {
            req.flash_old(&serde_json::Value::Object(old));
        }
        ok_redirect(path)
    }
    #[cfg(not(feature = "vld"))]
    {
        let _ = field;
        err_redirect(path, msg)
    }
}

fn fail(req: &Request, path: &str, status: u16, msg: &str) -> Response {
    if wants_json(req) {
        json_err(status, msg)
    } else {
        err_redirect(path, msg)
    }
}

fn map_err(req: &Request, path: &str, e: Error) -> Response {
    match e {
        Error::Unauthorized => fail(req, path, 401, "unauthorized"),
        Error::BadRequest(m) => fail(req, path, 400, &m),
        Error::NotFound => fail(req, path, 404, "not found"),
        Error::Response(r) => *r,
        other => fail(req, path, 500, &other.to_string()),
    }
}

async fn creds(req: &mut Request) -> Result<CredsForm> {
    #[cfg(feature = "vld")]
    {
        use crate::forms::*;
        use sova_vld::Validated;
        if let Some(v) = req.get::<Validated<RegisterForm>>() {
            let f = &v.0;
            return Ok(CredsForm {
                email: Some(f.email.clone()),
                name: Some(f.name.clone()),
                password: Some(f.password.clone()),
                password_confirmation: f.password_confirmation.clone(),
                csrf: f.csrf.clone(),
                ..Default::default()
            });
        }
        if let Some(v) = req.get::<Validated<LoginForm>>() {
            let f = &v.0;
            return Ok(CredsForm {
                email: Some(f.email.clone()),
                password: Some(f.password.clone()),
                csrf: f.csrf.clone(),
                ..Default::default()
            });
        }
        if let Some(v) = req.get::<Validated<ForgotForm>>() {
            let f = &v.0;
            return Ok(CredsForm {
                email: Some(f.email.clone()),
                csrf: f.csrf.clone(),
                ..Default::default()
            });
        }
        if let Some(v) = req.get::<Validated<ResetForm>>() {
            let f = &v.0;
            return Ok(CredsForm {
                email: Some(f.email.clone()),
                token: Some(f.token.clone()),
                password: Some(f.password.clone()),
                password_confirmation: Some(f.password_confirmation.clone()),
                csrf: f.csrf.clone(),
                ..Default::default()
            });
        }
        if let Some(v) = req.get::<Validated<ProfileForm>>() {
            let f = &v.0;
            return Ok(CredsForm {
                email: Some(f.email.clone()),
                name: Some(f.name.clone()),
                csrf: f.csrf.clone(),
                ..Default::default()
            });
        }
        if let Some(v) = req.get::<Validated<PasswordForm>>() {
            let f = &v.0;
            return Ok(CredsForm {
                current_password: Some(f.current_password.clone()),
                password: Some(f.password.clone()),
                password_confirmation: Some(f.password_confirmation.clone()),
                csrf: f.csrf.clone(),
                ..Default::default()
            });
        }
        if let Some(v) = req.get::<Validated<ConfirmPasswordForm>>() {
            let f = &v.0;
            return Ok(CredsForm {
                password: Some(f.password.clone()),
                csrf: f.csrf.clone(),
                ..Default::default()
            });
        }
        if let Some(v) = req.get::<Validated<TwoFactorCodeForm>>() {
            let f = &v.0;
            return Ok(CredsForm {
                code: f.code.clone(),
                recovery_code: f.recovery_code.clone(),
                csrf: f.csrf.clone(),
                ..Default::default()
            });
        }
        if let Some(v) = req.get::<Validated<DisableTwoFactorForm>>() {
            let f = &v.0;
            return Ok(CredsForm {
                password: f.password.clone(),
                current_password: f.current_password.clone(),
                csrf: f.csrf.clone(),
                ..Default::default()
            });
        }
    }
    read_creds(req).await
}

/// Minimal HTML register form (when `web_forms(true)`). CSRF field filled if session has one.
pub async fn register_form(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Registration)?;
    let csrf = session_csrf(&req);
    let action = req.path.clone();
    Ok(Response::html(format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>Register</title></head>
<body>
<form method="post" action="{action}">
<input type="hidden" name="csrf" value="{csrf}">
<label>Name <input name="name" required></label>
<label>Email <input name="email" type="email" required></label>
<label>Password <input name="password" type="password" required></label>
<label>Confirm <input name="password_confirmation" type="password" required></label>
<button type="submit">Register</button>
</form>
</body></html>"#
    )))
}

/// Minimal HTML login form (when `web_forms(true)`).
pub async fn login_form(req: Request) -> Result<Response> {
    let csrf = session_csrf(&req);
    let action = req.path.clone();
    Ok(Response::html(format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>Login</title></head>
<body>
<form method="post" action="{action}">
<input type="hidden" name="csrf" value="{csrf}">
<label>Email <input name="email" type="email" required></label>
<label>Password <input name="password" type="password" required></label>
<button type="submit">Login</button>
</form>
</body></html>"#
    )))
}

fn session_csrf(req: &Request) -> String {
    use sova_session::SessionExt;
    req.session().get("csrf").unwrap_or_default()
}

pub async fn register(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Registration)?;
    let form = creds(&mut req).await?;
    let email = form.email.as_deref().unwrap_or("").trim();
    let name = form.name.as_deref().unwrap_or("").trim();
    let password = form.password.as_deref().unwrap_or("");
    if let Some(confirm) = form.password_confirmation.as_deref() {
        if confirm != password {
            return Ok(field_fail(
                &req,
                state.paths.register.as_str(),
                "password_confirmation",
                "passwords do not match",
                400,
            ));
        }
    }
    if email.is_empty() || name.is_empty() {
        return Ok(field_fail(
            &req,
            state.paths.register.as_str(),
            "email",
            "name and email required",
            400,
        ));
    }
    let db = req.db().clone();
    let user = match store::register_user(&db, email, name, password).await {
        Ok(u) => u,
        Err(Error::Response(r)) => return Ok(*r),
        Err(Error::BadRequest(m)) if m.contains("password") => {
            return Ok(field_fail(&req, state.paths.register.as_str(), "password", &m, 400));
        }
        Err(e) if e.to_string().contains("already") || matches!(e, Error::Response(_)) => {
            return Ok(field_fail(&req, state.paths.register.as_str(), "email", "email already registered", 409));
        }
        Err(e) => return Ok(map_err(&req, state.paths.register.as_str(), e)),
    };
    let cu = store::load_current_user(&db, user.id)
        .await?
        .ok_or(Error::Internal("user missing after register".into()))?;

    if state.enabled(Feature::EmailVerification) {
        let token = make_verify_token(&state.secret, cu.id);
        let link = format!(
            "{}{}?token={}",
            state.public_url.trim_end_matches('/'),
            state.verify_path,
            urlencoding::encode(&token)
        );
        #[cfg(feature = "mail")]
        let _ = send_verify(&req, &cu.email, &link).await;
        #[cfg(not(feature = "mail"))]
        let _ = (token, link);
    }

    let mut req = req;
    if let Some(hook) = &state.after_register {
        req = hook(cu.clone(), req).await?;
    }

    #[cfg(feature = "activity")]
    log_event(
        &req,
        Some(cu.id),
        "user.registered",
        "user",
        cu.id,
        json!({ "email": cu.email }),
    )
    .await;

    finish_login(&mut req, cu, &state).await
}

pub async fn login(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    let form = creds(&mut req).await?;
    let email = form.email.as_deref().unwrap_or("").trim();
    let password = form.password.as_deref().unwrap_or("");
    let db = req.db().clone();
    let user = match store::attempt_login(&db, email, password).await {
        Ok(u) => u,
        Err(_) => {
            return Ok(field_fail(
                &req,
                &state.login_path,
                "email",
                "invalid credentials",
                401,
            ));
        }
    };

    if state.enabled(Feature::TwoFactor) && user.two_factor_confirmed_at.is_some() {
        req.session().set(PENDING_2FA_KEY, user.id.to_string());
        if wants_json(&req) {
            return Ok(json_ok(json!({ "two_factor": true })));
        }
        return Ok(ok_redirect(&state.two_factor_challenge_path));
    }

    let cu = store::load_current_user(&db, user.id)
        .await?
        .ok_or(Error::Unauthorized)?;
    #[cfg(feature = "activity")]
    log_event(
        &req,
        Some(cu.id),
        "user.login",
        "user",
        cu.id,
        json!({}),
    )
    .await;
    finish_login(&mut req, cu, &state).await
}

pub async fn logout(mut req: Request) -> Result<Response> {
    let home = req
        .try_state::<FortifyState>()
        .map(|s| s.home_path.clone())
        .unwrap_or_else(|| "/".into());
    #[cfg(feature = "activity")]
    if let Some(id) = req.get::<CurrentUser>().map(|u| u.id) {
        log_event(&req, Some(id), "user.logout", "user", id, json!({})).await;
    }
    use crate::guard::AuthExt;
    req.logout_user();
    if wants_json(&req) {
        return Ok(json_ok(json!({ "ok": true })));
    }
    Ok(ok_redirect(&home))
}

pub async fn forgot_password(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::ResetPasswords)?;
    let form = creds(&mut req).await?;
    let email = form.email.as_deref().unwrap_or("").trim().to_lowercase();
    let db = req.db().clone();
    if let Some(u) = store::find_user_by_email(&db, &email).await? {
        let raw = store::random_token();
        store::store_reset_token(&db, &u.email, &raw).await?;
        let link = format!(
            "{}{}?email={}&token={}",
            state.public_url.trim_end_matches('/'),
            state.paths.reset_password,
            urlencoding::encode(&u.email),
            urlencoding::encode(&raw)
        );
        #[cfg(feature = "mail")]
        let _ = send_reset(&req, &u.email, &link).await;
        #[cfg(not(feature = "mail"))]
        let _ = link;
    }
    if wants_json(&req) {
        return Ok(json_ok(json!({ "ok": true })));
    }
    Ok(ok_redirect(&format!("{}?sent=1", state.paths.forgot_password)))
}

pub async fn reset_password(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::ResetPasswords)?;
    let form = creds(&mut req).await?;
    let email = form.email.as_deref().unwrap_or("").trim();
    let token = form.token.as_deref().unwrap_or("");
    let password = form.password.as_deref().unwrap_or("");
    let confirm = form.password_confirmation.as_deref().unwrap_or(password);
    if password != confirm {
        return Ok(field_fail(
            &req,
            &state.paths.reset_password,
            "password_confirmation",
            "passwords do not match",
            400,
        ));
    }
    let db = req.db().clone();
    if let Err(e) = store::consume_reset_token(&db, email, token).await {
        return Ok(map_err(&req, &state.paths.reset_password, e));
    }
    let Some(u) = store::find_user_by_email(&db, email).await? else {
        return Ok(field_fail(
            &req,
            &state.paths.reset_password,
            "token",
            "invalid token",
            400,
        ));
    };
    if let Err(e) = store::set_password(&db, u.id, password).await {
        return Ok(map_err(&req, &state.paths.reset_password, e));
    }
    #[cfg(feature = "activity")]
    log_event(
        &req,
        None,
        "password.changed",
        "user",
        u.id,
        json!({ "via": "reset" }),
    )
    .await;
    if wants_json(&req) {
        return Ok(json_ok(json!({ "ok": true })));
    }
    Ok(ok_redirect(&format!("{}?reset=1", state.login_path)))
}

pub async fn verify_email(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::EmailVerification)?;
    let token = if let Some(t) = req.query("token") {
        t.to_string()
    } else {
        let form = creds(&mut req).await.unwrap_or_default();
        form.token.unwrap_or_default()
    };
    let id = match parse_verify_token(&state.secret, &token) {
        Ok(id) => id,
        Err(e) => return Ok(map_err(&req, &state.verify_path, e)),
    };
    let db = req.db().clone();
    store::mark_email_verified(&db, id).await?;
    #[cfg(feature = "activity")]
    log_event(
        &req,
        Some(id),
        "email.verified",
        "user",
        id,
        json!({}),
    )
    .await;
    if wants_json(&req) {
        return Ok(json_ok(json!({ "ok": true })));
    }
    Ok(ok_redirect(&format!("{}?verified=1", state.home_path)))
}

pub async fn resend_verification(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::EmailVerification)?;
    let user = req.require_current_user()?.clone();
    if user.email_verified {
        if wants_json(&req) {
            return Ok(json_ok(json!({ "ok": true })));
        }
        return Ok(ok_redirect(&state.home_path));
    }
    let token = make_verify_token(&state.secret, user.id);
    let link = format!(
        "{}{}?token={}",
        state.public_url.trim_end_matches('/'),
        state.verify_path,
        urlencoding::encode(&token)
    );
    #[cfg(feature = "mail")]
    let _ = send_verify(&req, &user.email, &link).await;
    #[cfg(not(feature = "mail"))]
    let _ = link;
    if wants_json(&req) {
        return Ok(json_ok(json!({ "ok": true })));
    }
    Ok(ok_redirect(&format!("{}?sent=1", state.verify_path)))
}

pub async fn update_profile(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::UpdateProfile)?;
    let user = req.require_current_user()?.clone();
    let form = creds(&mut req).await?;
    let name = form.name.as_deref().unwrap_or(&user.name);
    let email = form.email.as_deref().unwrap_or(&user.email);
    let db = req.db().clone();
    match store::update_profile(&db, user.id, name, email).await {
        Ok(_) => {}
        Err(e) if e.to_string().contains("already") => {
            return Ok(field_fail(
                &req,
                &state.profile_path,
                "email",
                "email already registered",
                409,
            ));
        }
        Err(e) => return Ok(map_err(&req, &state.profile_path, e)),
    }
    #[cfg(feature = "activity")]
    {
        let mut changed = serde_json::Map::new();
        if name != user.name {
            changed.insert(
                "name".into(),
                json!({ "old": user.name, "new": name }),
            );
        }
        if email != user.email {
            changed.insert(
                "email".into(),
                json!({ "old": user.email, "new": email }),
            );
        }
        if !changed.is_empty() {
            log_event(
                &req,
                Some(user.id),
                "profile.updated",
                "user",
                user.id,
                serde_json::Value::Object(changed),
            )
            .await;
        }
    }
    if let Some(cu) = store::load_current_user(&db, user.id).await? {
        req.set(sova_core::RateLimitIdentity(cu.id.to_string()));
        req.set(cu);
    }
    if wants_json(&req) {
        return Ok(json_ok(json!({ "ok": true })));
    }
    Ok(ok_redirect(&state.profile_path))
}

pub async fn update_password(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::UpdatePasswords)?;
    let user = req.require_current_user()?.clone();
    let form = creds(&mut req).await?;
    let current = form.current_password.as_deref().unwrap_or("");
    let password = form.password.as_deref().unwrap_or("");
    let confirm = form.password_confirmation.as_deref().unwrap_or(password);
    if password != confirm {
        return Ok(field_fail(
            &req,
            &state.profile_path,
            "password_confirmation",
            "passwords do not match",
            400,
        ));
    }
    let db = req.db().clone();
    let Some(row) = store::find_user_by_id(&db, user.id).await? else {
        return Err(Error::Unauthorized);
    };
    let Some(hash) = row.password_hash.as_deref() else {
        return Err(Error::Unauthorized);
    };
    if !verify_password(current, hash)? {
        return Ok(field_fail(
            &req,
            &state.profile_path,
            "current_password",
            "current password incorrect",
            400,
        ));
    }
    store::set_password(&db, user.id, password).await?;
    #[cfg(feature = "activity")]
    log_event(
        &req,
        Some(user.id),
        "password.changed",
        "user",
        user.id,
        json!({ "via": "profile" }),
    )
    .await;
    if wants_json(&req) {
        return Ok(json_ok(json!({ "ok": true })));
    }
    Ok(ok_redirect(&format!("{}?password=1", state.profile_path)))
}

pub async fn confirm_password(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    let user = req.require_current_user()?.clone();
    let form = creds(&mut req).await?;
    let password = form.password.as_deref().unwrap_or("");
    let db = req.db().clone();
    let Some(row) = store::find_user_by_id(&db, user.id).await? else {
        return Err(Error::Unauthorized);
    };
    let Some(hash) = row.password_hash.as_deref() else {
        return Err(Error::Unauthorized);
    };
    if !verify_password(password, hash)? {
        return Ok(field_fail(
            &req,
            &state.confirm_password_path,
            "password",
            "password incorrect",
            400,
        ));
    }
    guard::mark_password_confirmed(&req);
    if wants_json(&req) {
        return Ok(json_ok(json!({ "confirmed": true })));
    }
    Ok(ok_redirect(&state.profile_path))
}

pub async fn confirmed_password_status(req: Request) -> Result<Response> {
    let _ = req.require_current_user()?;
    Ok(json_ok(json!({
        "confirmed": guard::is_password_confirmed(&req),
    })))
}

pub async fn two_factor_enable(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::TwoFactor)?;
    let user = req.require_current_user()?.clone();
    let secret = two_factor::generate_secret();
    let codes = generate_recovery_codes(8);
    let codes_json = hash_recovery_codes(&codes)?;
    let db = req.db().clone();
    store::enable_2fa_secret(&db, user.id, &secret, &codes_json).await?;
    #[cfg(feature = "activity")]
    log_event(
        &req,
        Some(user.id),
        "2fa.enabled",
        "user",
        user.id,
        json!({}),
    )
    .await;
    let url = two_factor::otpauth_url(&secret, &state.app_name, &user.email)?;
    if wants_json(&req) {
        return Ok(json_ok(json!({
            "secret": secret,
            "otpauth_url": url,
            "recovery_codes": codes,
        })));
    }
    req.session().set("fortify:2fa_secret", secret);
    req.session().set(
        "fortify:2fa_codes",
        serde_json::to_string(&codes).unwrap_or_default(),
    );
    req.session().set("fortify:2fa_url", url);
    Ok(ok_redirect(&state.two_factor_path))
}

pub async fn two_factor_confirm(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::TwoFactor)?;
    let user = req.require_current_user()?.clone();
    let form = creds(&mut req).await?;
    let code = form.code.as_deref().unwrap_or("");
    let db = req.db().clone();
    let Some(row) = store::find_user_by_id(&db, user.id).await? else {
        return Err(Error::Unauthorized);
    };
    let Some(secret) = row.two_factor_secret.as_deref() else {
        return Ok(fail(
            &req,
            &state.two_factor_path,
            400,
            "2FA not started",
        ));
    };
    if !two_factor::verify_code(secret, code)? {
        return Ok(field_fail(
            &req,
            &state.two_factor_path,
            "code",
            "invalid code",
            400,
        ));
    }
    store::confirm_2fa(&db, user.id).await?;
    #[cfg(feature = "activity")]
    log_event(
        &req,
        Some(user.id),
        "2fa.confirmed",
        "user",
        user.id,
        json!({}),
    )
    .await;
    req.session().remove("fortify:2fa_secret");
    req.session().remove("fortify:2fa_codes");
    req.session().remove("fortify:2fa_url");
    if wants_json(&req) {
        return Ok(json_ok(json!({ "ok": true })));
    }
    Ok(ok_redirect(&format!("{}?enabled=1", state.two_factor_path)))
}

pub async fn two_factor_disable(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::TwoFactor)?;
    let user = req.require_current_user()?.clone();
    let form = creds(&mut req).await?;
    let password = form
        .password
        .as_deref()
        .or(form.current_password.as_deref())
        .unwrap_or("");
    let db = req.db().clone();
    let Some(row) = store::find_user_by_id(&db, user.id).await? else {
        return Err(Error::Unauthorized);
    };
    let Some(hash) = row.password_hash.as_deref() else {
        return Err(Error::Unauthorized);
    };
    if !verify_password(password, hash)? {
        return Ok(field_fail(
            &req,
            &state.two_factor_path,
            "password",
            "password incorrect",
            400,
        ));
    }
    store::disable_2fa(&db, user.id).await?;
    #[cfg(feature = "activity")]
    log_event(
        &req,
        Some(user.id),
        "2fa.disabled",
        "user",
        user.id,
        json!({}),
    )
    .await;
    if wants_json(&req) {
        return Ok(json_ok(json!({ "ok": true })));
    }
    Ok(ok_redirect(&format!("{}?disabled=1", state.two_factor_path)))
}

pub async fn two_factor_challenge(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::TwoFactor)?;
    let Some(pending) = req.session().get(PENDING_2FA_KEY) else {
        return Ok(fail(&req, &state.login_path, 401, "no pending 2FA"));
    };
    let user_id: i64 = pending.parse().map_err(|_| Error::Unauthorized)?;
    let form = creds(&mut req).await?;
    let db = req.db().clone();
    let Some(row) = store::find_user_by_id(&db, user_id).await? else {
        return Err(Error::Unauthorized);
    };

    let mut ok = false;
    if let Some(code) = form.code.as_deref().filter(|c| !c.is_empty()) {
        if let Some(secret) = row.two_factor_secret.as_deref() {
            ok = two_factor::verify_code(secret, code)?;
        }
    } else if let Some(rec) = form.recovery_code.as_deref().filter(|c| !c.is_empty()) {
        if let Some(codes) = row.two_factor_recovery_codes.as_deref() {
            if let Some(updated) = consume_recovery_code(codes, rec)? {
                store::set_recovery_codes(&db, user_id, &updated).await?;
                ok = true;
            }
        }
    }
    if !ok {
        return Ok(field_fail(
            &req,
            state.two_factor_challenge_path.as_str(),
            "code",
            "invalid code",
            401,
        ));
    }
    req.session().remove(PENDING_2FA_KEY);
    let cu = store::load_current_user(&db, user_id)
        .await?
        .ok_or(Error::Unauthorized)?;
    #[cfg(feature = "activity")]
    log_event(
        &req,
        Some(cu.id),
        "user.login",
        "user",
        cu.id,
        json!({ "via": "2fa" }),
    )
    .await;
    finish_login(&mut req, cu, &state).await
}

pub async fn two_factor_qr_code(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::TwoFactor)?;
    let user = req.require_current_user()?.clone();
    let db = req.db().clone();
    let Some(row) = store::find_user_by_id(&db, user.id).await? else {
        return Err(Error::Unauthorized);
    };
    let Some(secret) = row.two_factor_secret.as_deref() else {
        return Err(Error::NotFound);
    };
    let url = two_factor::otpauth_url(secret, &state.app_name, &user.email)?;
    Ok(json_ok(json!({ "otpauth_url": url })))
}

pub async fn two_factor_secret_key(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::TwoFactor)?;
    let user = req.require_current_user()?.clone();
    let db = req.db().clone();
    let Some(row) = store::find_user_by_id(&db, user.id).await? else {
        return Err(Error::Unauthorized);
    };
    let Some(secret) = row.two_factor_secret else {
        return Err(Error::NotFound);
    };
    Ok(json_ok(json!({ "secret_key": secret })))
}

pub async fn two_factor_recovery_codes_get(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::TwoFactor)?;
    let _ = req.require_current_user()?;
    // Hashed at rest — only available from session after enable.
    let Some(raw) = req.session().get("fortify:2fa_codes") else {
        return Err(Error::custom(
            404,
            Json(json!({
                "error": "recovery codes unavailable; regenerate with POST"
            })),
        ));
    };
    let codes: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| json!([]));
    Ok(json_ok(json!({ "recovery_codes": codes })))
}

pub async fn two_factor_recovery_codes_regen(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::TwoFactor)?;
    let user = req.require_current_user()?.clone();
    let db = req.db().clone();
    let Some(row) = store::find_user_by_id(&db, user.id).await? else {
        return Err(Error::Unauthorized);
    };
    if row.two_factor_confirmed_at.is_none() {
        return Err(Error::BadRequest("2FA not enabled".into()));
    }
    let codes = generate_recovery_codes(8);
    let codes_json = hash_recovery_codes(&codes)?;
    store::set_recovery_codes(&db, user.id, &codes_json).await?;
    req.session().set(
        "fortify:2fa_codes",
        serde_json::to_string(&codes).unwrap_or_default(),
    );
    Ok(json_ok(json!({ "recovery_codes": codes })))
}

fn parse_id(req: &Request) -> Result<i64> {
    req.param("id")
        .ok_or_else(|| Error::BadRequest("missing id".into()))?
        .parse()
        .map_err(|_| Error::BadRequest("invalid id".into()))
}

fn role_json(r: &crate::entity::role::Model) -> serde_json::Value {
    json!({ "id": r.id, "name": r.name, "slug": r.slug })
}

fn perm_json(p: &crate::entity::permission::Model) -> serde_json::Value {
    json!({ "id": p.id, "name": p.name, "slug": p.slug })
}

#[derive(Debug, Default, serde::Deserialize)]
struct NameSlugBody {
    name: Option<String>,
    slug: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct IdsBody {
    #[serde(default)]
    permission_ids: Vec<i64>,
    #[serde(default)]
    role_ids: Vec<i64>,
}

pub async fn roles_list(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let db = req.db().clone();
    let roles = store::list_roles(&db).await?;
    Ok(json_ok(json!({
        "roles": roles.iter().map(role_json).collect::<Vec<_>>(),
    })))
}

pub async fn roles_show(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let id = parse_id(&req)?;
    let db = req.db().clone();
    let Some(r) = store::find_role(&db, id).await? else {
        return Err(Error::NotFound);
    };
    let permission_ids = store::role_permission_ids(&db, id).await?;
    Ok(json_ok(json!({
        "role": role_json(&r),
        "permission_ids": permission_ids,
    })))
}

pub async fn roles_create(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let body: NameSlugBody = req.json().await.unwrap_or_default();
    let name = body.name.as_deref().unwrap_or("").trim();
    let slug = body.slug.as_deref().unwrap_or("").trim();
    let db = req.db().clone();
    let r = store::create_role(&db, name, slug).await?;
    #[cfg(feature = "activity")]
    {
        let actor = req.get::<CurrentUser>().map(|u| u.id);
        log_event(
            &req,
            actor,
            "role.created",
            "role",
            r.id,
            json!({ "name": r.name, "slug": r.slug }),
        )
        .await;
    }
    Ok((201, Json(json!({ "role": role_json(&r) }))).into_response())
}

pub async fn roles_update(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let id = parse_id(&req)?;
    let body: NameSlugBody = req.json().await.unwrap_or_default();
    let db = req.db().clone();
    let r = store::update_role(
        &db,
        id,
        body.name.as_deref(),
        body.slug.as_deref(),
        state.allow_system_role_delete,
    )
    .await?;
    #[cfg(feature = "activity")]
    {
        let actor = req.get::<CurrentUser>().map(|u| u.id);
        log_event(
            &req,
            actor,
            "role.updated",
            "role",
            r.id,
            json!({ "name": r.name, "slug": r.slug }),
        )
        .await;
    }
    Ok(json_ok(json!({ "role": role_json(&r) })))
}

pub async fn roles_delete(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let id = parse_id(&req)?;
    let db = req.db().clone();
    store::delete_role(&db, id, state.allow_system_role_delete).await?;
    #[cfg(feature = "activity")]
    {
        let actor = req.get::<CurrentUser>().map(|u| u.id);
        log_event(&req, actor, "role.deleted", "role", id, json!({})).await;
    }
    Ok(json_ok(json!({ "ok": true })))
}

pub async fn roles_sync_permissions(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let id = parse_id(&req)?;
    let body: IdsBody = req.json().await.unwrap_or_default();
    let db = req.db().clone();
    store::sync_role_permissions(&db, id, &body.permission_ids).await?;
    #[cfg(feature = "activity")]
    {
        let actor = req.get::<CurrentUser>().map(|u| u.id);
        log_event(
            &req,
            actor,
            "role.permissions.synced",
            "role",
            id,
            json!({ "permission_ids": body.permission_ids }),
        )
        .await;
    }
    Ok(json_ok(json!({
        "ok": true,
        "permission_ids": body.permission_ids,
    })))
}

pub async fn permissions_list(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let db = req.db().clone();
    let perms = store::list_permissions(&db).await?;
    Ok(json_ok(json!({
        "permissions": perms.iter().map(perm_json).collect::<Vec<_>>(),
    })))
}

pub async fn permissions_create(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let body: NameSlugBody = req.json().await.unwrap_or_default();
    let db = req.db().clone();
    let p = store::create_permission(
        &db,
        body.name.as_deref().unwrap_or(""),
        body.slug.as_deref().unwrap_or(""),
    )
    .await?;
    #[cfg(feature = "activity")]
    {
        let actor = req.get::<CurrentUser>().map(|u| u.id);
        log_event(
            &req,
            actor,
            "permission.created",
            "permission",
            p.id,
            json!({ "name": p.name, "slug": p.slug }),
        )
        .await;
    }
    Ok((201, Json(json!({ "permission": perm_json(&p) }))).into_response())
}

pub async fn permissions_update(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let id = parse_id(&req)?;
    let body: NameSlugBody = req.json().await.unwrap_or_default();
    let db = req.db().clone();
    let p = store::update_permission(&db, id, body.name.as_deref(), body.slug.as_deref()).await?;
    #[cfg(feature = "activity")]
    {
        let actor = req.get::<CurrentUser>().map(|u| u.id);
        log_event(
            &req,
            actor,
            "permission.updated",
            "permission",
            p.id,
            json!({ "name": p.name, "slug": p.slug }),
        )
        .await;
    }
    Ok(json_ok(json!({ "permission": perm_json(&p) })))
}

pub async fn permissions_delete(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let id = parse_id(&req)?;
    let db = req.db().clone();
    store::delete_permission(&db, id).await?;
    #[cfg(feature = "activity")]
    {
        let actor = req.get::<CurrentUser>().map(|u| u.id);
        log_event(
            &req,
            actor,
            "permission.deleted",
            "permission",
            id,
            json!({}),
        )
        .await;
    }
    Ok(json_ok(json!({ "ok": true })))
}

pub async fn user_roles_list(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let id = parse_id(&req)?;
    let db = req.db().clone();
    let role_ids = store::user_role_ids(&db, id).await?;
    Ok(json_ok(json!({ "user_id": id, "role_ids": role_ids })))
}

pub async fn user_roles_sync(mut req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    require_feature(&state, Feature::Roles)?;
    req.require_permission("users.manage")?;
    let id = parse_id(&req)?;
    let body: IdsBody = req.json().await.unwrap_or_default();
    let db = req.db().clone();
    store::set_user_roles(&db, id, &body.role_ids).await?;
    #[cfg(feature = "activity")]
    {
        let actor = req.get::<CurrentUser>().map(|u| u.id);
        log_event(
            &req,
            actor,
            "user.roles.synced",
            "user",
            id,
            json!({ "role_ids": body.role_ids }),
        )
        .await;
    }
    Ok(json_ok(json!({ "ok": true, "role_ids": body.role_ids })))
}

pub async fn me(req: Request) -> Result<Response> {
    let user = req.require_current_user()?;
    Ok(json_ok(json!({
        "id": user.id,
        "email": user.email,
        "name": user.name,
        "avatar_path": user.avatar_path,
        "email_verified": user.email_verified,
        "two_factor_enabled": user.two_factor_enabled,
        "roles": user.roles,
        "permissions": user.permissions,
    })))
}

/// GET profile: JSON for API/SPA; HTML redirect to configured profile_path.
pub async fn profile_get(req: Request) -> Result<Response> {
    let state = req.state::<FortifyState>().clone();
    if wants_json(&req) || req.path.contains("/api/") {
        return me(req).await;
    }
    let _ = req.require_current_user()?;
    Ok(ok_redirect(&state.profile_path))
}

async fn finish_login(
    req: &mut Request,
    cu: CurrentUser,
    state: &FortifyState,
) -> Result<Response> {
    use crate::guard::AuthExt;
    req.login_user(cu.clone());
    if wants_json(req) {
        return Ok(json_ok(json!({
            "id": cu.id,
            "email": cu.email,
            "name": cu.name,
        })));
    }
    Ok(ok_redirect(&state.home_path))
}
