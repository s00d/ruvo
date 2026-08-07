//! Auth guards and request helpers.

use crate::state::{
    wants_json, FortifyState, PASSWORD_CONFIRMED_AT, PASSWORD_CONFIRM_TTL_SECS,
};
use crate::store::CurrentUser;
use crate::token::now_secs;
use ruvo_core::extend::{named, MwEntry};
use ruvo_core::{with_state, Error, IntoResponse, Next, Redirect, Request, Response, Result};
use ruvo_passport::PassportExt;
use ruvo_session::SessionExt;

/// Require login: redirect HTML to configured login path, else 401.
pub fn fortify_guard(login_path: impl Into<String>) -> MwEntry {
    let login = login_path.into();
    named(
        "fortify-guard",
        with_state(login, |login, req, next| async move {
            if req.get::<CurrentUser>().is_some() || req.is_authenticated() {
                if req.get::<CurrentUser>().is_none() {
                    return reject(&req, login.as_str()).into_response();
                }
                return next(req).await;
            }
            reject(&req, login.as_str()).into_response()
        }),
    )
}

/// Like [`fortify_guard`], but reads `FortifyState.login_path` per request.
pub fn fortify_guard_from_state() -> MwEntry {
    named("fortify-guard", |req: Request, next: Next| async move {
        let login = req
            .try_state::<FortifyState>()
            .map(|s| s.login_path.clone())
            .unwrap_or_else(|| "/login".into());
        if req.get::<CurrentUser>().is_some() || req.is_authenticated() {
            if req.get::<CurrentUser>().is_none() {
                return reject(&req, &login).into_response();
            }
            return next(req).await;
        }
        reject(&req, &login).into_response()
    })
}

fn reject(req: &Request, login: &str) -> Response {
    if wants_json(req) {
        Error::Unauthorized.into_response()
    } else {
        Redirect::see_other(login).into_response()
    }
}

/// Require verified email (redirect or 403 JSON).
pub fn verified(verify_path: impl Into<String>) -> MwEntry {
    let path = verify_path.into();
    named(
        "fortify-verified",
        with_state(path, |path, req, next| async move {
            match req.get::<CurrentUser>() {
                Some(u) if u.email_verified => next(req).await,
                Some(_) => {
                    if wants_json(&req) {
                        Error::custom(403, "Email not verified").into_response()
                    } else {
                        Redirect::see_other(path.as_str()).into_response()
                    }
                }
                None => Error::Unauthorized.into_response(),
            }
        }),
    )
}

pub fn verified_from_state() -> MwEntry {
    named("fortify-verified", |req: Request, next: Next| async move {
        let path = req
            .try_state::<FortifyState>()
            .map(|s| s.verify_path.clone())
            .unwrap_or_else(|| "/email/verify".into());
        match req.get::<CurrentUser>() {
            Some(u) if u.email_verified => next(req).await,
            Some(_) => {
                if wants_json(&req) {
                    Error::custom(403, "Email not verified").into_response()
                } else {
                    Redirect::see_other(path.as_str()).into_response()
                }
            }
            None => Error::Unauthorized.into_response(),
        }
    })
}

/// Require recent password confirmation (session TTL).
pub fn password_confirmed(confirm_path: impl Into<String>) -> MwEntry {
    let path = confirm_path.into();
    named(
        "fortify-password-confirmed",
        with_state(path, |path, req, next| async move {
            if is_password_confirmed(&req) {
                return next(req).await;
            }
            if wants_json(&req) {
                Error::custom(423, "Password confirmation required").into_response()
            } else {
                Redirect::see_other(path.as_str()).into_response()
            }
        }),
    )
}

pub fn password_confirmed_from_state() -> MwEntry {
    named(
        "fortify-password-confirmed",
        |req: Request, next: Next| async move {
            let path = req
                .try_state::<FortifyState>()
                .map(|s| s.confirm_password_path.clone())
                .unwrap_or_else(|| "/user/confirm-password".into());
            if is_password_confirmed(&req) {
                return next(req).await;
            }
            if wants_json(&req) {
                Error::custom(423, "Password confirmation required").into_response()
            } else {
                Redirect::see_other(path.as_str()).into_response()
            }
        },
    )
}

pub fn is_password_confirmed(req: &Request) -> bool {
    let Some(raw) = req.session().get(PASSWORD_CONFIRMED_AT) else {
        return false;
    };
    let Ok(at) = raw.parse::<u64>() else {
        return false;
    };
    now_secs().saturating_sub(at) <= PASSWORD_CONFIRM_TTL_SECS
}

pub fn mark_password_confirmed(req: &Request) {
    req.session()
        .set(PASSWORD_CONFIRMED_AT, now_secs().to_string());
}

/// Require a permission slug (`admin` role bypasses via [`CurrentUser::has_permission`]).
pub fn permission(slug: impl Into<String>) -> MwEntry {
    let slug = slug.into();
    named(
        format!("fortify-permission:{slug}"),
        with_state(slug, |slug, req, next| async move {
            match req.get::<CurrentUser>() {
                Some(u) if u.has_permission(&slug) => next(req).await,
                Some(_) => Error::custom(403, "Forbidden").into_response(),
                None => Error::Unauthorized.into_response(),
            }
        }),
    )
}

/// Require a role slug.
pub fn role(slug: impl Into<String>) -> MwEntry {
    let slug = slug.into();
    named(
        format!("fortify-role:{slug}"),
        with_state(slug, |slug, req, next| async move {
            match req.get::<CurrentUser>() {
                Some(u) if u.has_role(&slug) => next(req).await,
                Some(_) => Error::custom(403, "Forbidden").into_response(),
                None => Error::Unauthorized.into_response(),
            }
        }),
    )
}

/// Request helpers for RBAC (`CurrentUser` from Fortify / Passport session).
pub trait AuthExt {
    fn current_user(&self) -> Option<&CurrentUser>;
    fn require_current_user(&self) -> Result<&CurrentUser>;
    /// Alias for [`Self::require_current_user`].
    fn profile(&self) -> Result<&CurrentUser>;
    fn require_permission(&self, slug: &str) -> Result<&CurrentUser>;
    fn require_role(&self, slug: &str) -> Result<&CurrentUser>;
    fn password_confirmed(&self) -> bool;
}

impl AuthExt for Request {
    fn current_user(&self) -> Option<&CurrentUser> {
        self.get::<CurrentUser>()
    }

    fn require_current_user(&self) -> Result<&CurrentUser> {
        self.get::<CurrentUser>().ok_or(Error::Unauthorized)
    }

    fn profile(&self) -> Result<&CurrentUser> {
        self.require_current_user()
    }

    fn require_permission(&self, slug: &str) -> Result<&CurrentUser> {
        let u = self.require_current_user()?;
        if u.has_permission(slug) {
            Ok(u)
        } else {
            Err(Error::custom(403, "Forbidden"))
        }
    }

    fn require_role(&self, slug: &str) -> Result<&CurrentUser> {
        let u = self.require_current_user()?;
        if u.has_role(slug) {
            Ok(u)
        } else {
            Err(Error::custom(403, "Forbidden"))
        }
    }

    fn password_confirmed(&self) -> bool {
        is_password_confirmed(self)
    }
}
