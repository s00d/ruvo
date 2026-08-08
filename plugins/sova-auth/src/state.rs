//! Shared Fortify runtime state + form helpers.

use crate::feature::Feature;
use crate::paths::FortifyPaths;
use crate::store::CurrentUser;
use sova_core::extend::BoxFuture;
use sova_core::{Error, Request, Result};
use std::collections::HashSet;
use std::sync::Arc;

pub const PENDING_2FA_KEY: &str = "fortify:pending_2fa";
pub const PASSWORD_CONFIRMED_AT: &str = "fortify:password_confirmed_at";
/// Default confirm-password TTL (seconds).
pub const PASSWORD_CONFIRM_TTL_SECS: u64 = 3 * 60 * 60;

pub type AfterRegisterFn =
    Arc<dyn Fn(CurrentUser, Request) -> BoxFuture<Result<Request>> + Send + Sync>;

#[derive(Clone)]
pub struct FortifyState {
    pub features: HashSet<Feature>,
    pub secret: String,
    pub public_url: String,
    pub app_name: String,
    pub home_path: String,
    pub login_path: String,
    pub profile_path: String,
    pub verify_path: String,
    pub confirm_password_path: String,
    pub two_factor_challenge_path: String,
    pub two_factor_path: String,
    pub paths: FortifyPaths,
    pub allow_system_role_delete: bool,
    pub after_register: Option<AfterRegisterFn>,
}

impl FortifyState {
    pub fn enabled(&self, f: Feature) -> bool {
        self.features.contains(&f)
    }
}

pub fn require_feature(state: &FortifyState, f: Feature) -> Result<()> {
    if state.enabled(f) {
        Ok(())
    } else {
        Err(Error::NotFound)
    }
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct CredsForm {
    pub email: Option<String>,
    pub password: Option<String>,
    pub name: Option<String>,
    pub token: Option<String>,
    pub code: Option<String>,
    pub recovery_code: Option<String>,
    pub current_password: Option<String>,
    pub password_confirmation: Option<String>,
    #[allow(dead_code)]
    pub csrf: Option<String>,
}

pub async fn read_creds(req: &mut Request) -> Result<CredsForm> {
    if req
        .header("content-type")
        .map(|ct| ct.contains("application/json"))
        .unwrap_or(false)
    {
        return req.json().await;
    }
    req.form().await
}

pub fn wants_json(req: &Request) -> bool {
    req.header("accept")
        .map(|v| v.contains("application/json"))
        .unwrap_or(false)
        || req.path.contains("/api/")
}
