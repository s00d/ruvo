//! Batteries-included JWT auth plugin (feature `jwt`).

use crate::extract::Extract;
use crate::jwt::Jwt;
use crate::store::{
    authenticate, find_user_by_id, register_user, revoke_refresh, user_for_refresh, AuthUser,
    TokenPair,
};
use crate::AuthExt;
use ruvo_core::extend::{named, MwEntry};
use ruvo_core::{App, Error, Json, Plugin, Request, Response, Result, Router};
use ruvo_db::DbExt;
use serde::Deserialize;
use std::sync::Arc;

const DEFAULT_ACCESS_TTL: u64 = 900;
const DEFAULT_REFRESH_TTL: u64 = 604_800;

/// Shared config placed in app state by [`JwtAuth`].
#[derive(Clone)]
pub struct JwtAuthState {
    pub jwt: Jwt,
    pub access_ttl: u64,
    pub refresh_ttl: u64,
}

/// Full JWT auth: register/login/refresh/logout routes + [`JwtAuth::guard`].
pub struct JwtAuth {
    jwt: Jwt,
    access_ttl: u64,
    refresh_ttl: u64,
    mount: String,
}

impl JwtAuth {
    pub fn hs256(secret: impl Into<String>) -> Self {
        Self {
            jwt: Jwt::hs256(secret),
            access_ttl: DEFAULT_ACCESS_TTL,
            refresh_ttl: DEFAULT_REFRESH_TTL,
            mount: "/auth".into(),
        }
    }

    /// `JWT_SECRET` required; optional `JWT_ACCESS_TTL` / `JWT_REFRESH_TTL` (seconds).
    pub fn from_env() -> Self {
        let secret = std::env::var("JWT_SECRET").unwrap_or_default();
        let mut auth = Self::hs256(secret);
        if let Ok(v) = std::env::var("JWT_ACCESS_TTL") {
            if let Ok(n) = v.parse() {
                auth.access_ttl = n;
            }
        }
        if let Ok(v) = std::env::var("JWT_REFRESH_TTL") {
            if let Ok(n) = v.parse() {
                auth.refresh_ttl = n;
            }
        }
        auth
    }

    pub fn mount(mut self, path: impl Into<String>) -> Self {
        self.mount = path.into();
        self
    }

    pub fn access_ttl(mut self, secs: u64) -> Self {
        self.access_ttl = secs;
        self
    }

    pub fn refresh_ttl(mut self, secs: u64) -> Self {
        self.refresh_ttl = secs;
        self
    }

    /// Middleware: `Authorization: Bearer <access>` → `req.set(AuthUser)`.
    pub fn guard() -> MwEntry {
        named(
            "jwt-auth-guard",
            |mut req: Request, next: ruvo_core::Next| async move {
                let Some(creds) = Extract::bearer().run(&req) else {
                    return Error::Unauthorized.into_response();
                };
                let state = Arc::clone(&req.state::<JwtAuthState>());
                let claims = match state.jwt.decode_access(creds.value()) {
                    Ok(c) => c,
                    Err(_) => return Error::Unauthorized.into_response(),
                };
                let Ok(id) = claims.sub.parse::<i64>() else {
                    return Error::Unauthorized.into_response();
                };
                let db = req.db().clone();
                match find_user_by_id(&db, id).await {
                    Ok(Some(u)) => {
                        let auth = AuthUser::from(&u);
                        req.set(crate::Authenticated {
                            id: auth.id.to_string(),
                        });
                        req.set(auth);
                        next(req).await
                    }
                    Ok(None) => Error::Unauthorized.into_response(),
                    Err(e) => e.into_response(),
                }
            },
        )
    }
}

impl Plugin for JwtAuth {
    fn id(&self) -> &'static str {
        "jwt-auth"
    }

    fn requires(&self) -> &'static [&'static str] {
        &["db"]
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("JWT Auth")
            .description("Users + access/refresh JWT (register/login/refresh/logout)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        if self.jwt.secret().is_empty() {
            app.on_startup(|_state| async {
                Err(Error::Internal(
                    "JWT_SECRET is empty; set it before installing JwtAuth".into(),
                ))
            });
            return;
        }

        let state = JwtAuthState {
            jwt: self.jwt,
            access_ttl: self.access_ttl,
            refresh_ttl: self.refresh_ttl,
        };
        app.state(state);

        let mut r = Router::new();
        r.post("/register", register_handler);
        r.post("/login", login_handler);
        r.post("/refresh", refresh_handler);
        r.post("/logout", logout_handler);
        app.mount(&self.mount, r);
    }
}

#[derive(Debug, Deserialize)]
struct CredentialsBody {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct RefreshBody {
    refresh_token: String,
}

async fn issue_pair(req: &Request, user: &crate::entity::user::Model) -> Result<TokenPair> {
    crate::store::issue_token_pair(req, user).await
}

async fn register_handler(mut req: Request) -> Result<(u16, Json<TokenPair>)> {
    let body: CredentialsBody = req.json().await?;
    let user = register_user(req.db(), &body.email, &body.password).await?;
    let pair = issue_pair(&req, &user).await?;
    Ok((201, Json(pair)))
}

async fn login_handler(mut req: Request) -> Result<Json<TokenPair>> {
    let body: CredentialsBody = req.json().await?;
    let user = authenticate(req.db(), &body.email, &body.password).await?;
    Ok(Json(issue_pair(&req, &user).await?))
}

async fn refresh_handler(mut req: Request) -> Result<Json<TokenPair>> {
    let body: RefreshBody = req.json().await?;
    let user = user_for_refresh(req.db(), &body.refresh_token).await?;
    let _ = revoke_refresh(req.db(), &body.refresh_token).await?;
    Ok(Json(issue_pair(&req, &user).await?))
}

async fn logout_handler(mut req: Request) -> Result<Response> {
    let body: RefreshBody = req.json().await?;
    let _ = revoke_refresh(req.db(), &body.refresh_token).await?;
    Ok(Response::text("ok"))
}

/// Convenience: require [`AuthUser`] in handlers after [`JwtAuth::guard`].
pub trait JwtAuthExt {
    fn auth_user(&self) -> Option<&AuthUser>;
    fn require_auth_user(&self) -> Result<&AuthUser>;
}

impl JwtAuthExt for Request {
    fn auth_user(&self) -> Option<&AuthUser> {
        self.user::<AuthUser>()
    }

    fn require_auth_user(&self) -> Result<&AuthUser> {
        self.require_user::<AuthUser>()
    }
}
