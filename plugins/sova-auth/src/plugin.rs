//! Fortify plugin: Passport session + JSON API (web forms opt-in).
//!
//! Install order (runtime `requires`): **`db` → `session` → Fortify**.
//! Add **`mail`** before Fortify when enabling `EmailVerification` or `ResetPasswords`.
//! Passport is installed inside Fortify. CSRF stays app-level (e.g. cabinet).

use crate::actions;
use crate::feature::Feature;
use crate::guard::{self, fortify_guard, fortify_guard_from_state};
use crate::paths::FortifyPaths;
use crate::state::{AfterRegisterFn, FortifyState};
use crate::store::{self, CurrentUser};
use sova_core::extend::{BoxFuture, MwEntry};
use sova_core::{App, Error, Plugin, RateLimitIdentity, Request, Result, Router};
use sova_db::DbExt;
use sova_passport::Passport;
use sova_rate_limit::RateLimit;
use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;

#[cfg(feature = "vld")]
use sova_vld::ValidateRouteExt;

/// Laravel Fortify-style auth layer on top of Passport + Mail + Db.
pub struct Fortify {
    features: HashSet<Feature>,
    /// Opt-in HTML form GET/POST routes (Laravel-style paths). Default: off.
    web_forms: bool,
    mount: String,
    api_mount: Option<String>,
    paths: FortifyPaths,
    secret: String,
    public_url: String,
    app_name: String,
    home_path: String,
    login_path: String,
    profile_path: String,
    verify_path: String,
    confirm_password_path: String,
    two_factor_challenge_path: String,
    two_factor_path: String,
    /// Allow DELETE of seed roles `admin` / `user`.
    allow_system_role_delete: bool,
    after_register: Option<AfterRegisterFn>,
}

impl Fortify {
    pub fn new() -> Self {
        Self {
            // Registration-only by default — mail-backed features are opt-in.
            features: [Feature::Registration].into_iter().collect(),
            web_forms: false,
            mount: "/".into(),
            api_mount: Some("/api/auth".into()),
            paths: FortifyPaths::default(),
            secret: std::env::var("FORTIFY_SECRET")
                .or_else(|_| std::env::var("APP_KEY"))
                .unwrap_or_else(|_| "dev-fortify-secret-change-me".into()),
            public_url: std::env::var("PUBLIC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3000".into()),
            app_name: std::env::var("APP_NAME").unwrap_or_else(|_| "Sova".into()),
            home_path: "/cabinet".into(),
            login_path: "/login".into(),
            profile_path: "/user/profile".into(),
            verify_path: "/email/verify".into(),
            confirm_password_path: "/user/confirm-password".into(),
            two_factor_challenge_path: "/two-factor-challenge".into(),
            two_factor_path: "/user/two-factor-authentication".into(),
            allow_system_role_delete: false,
            after_register: None,
        }
    }

    pub fn features(mut self, features: impl IntoIterator<Item = Feature>) -> Self {
        self.features = features.into_iter().collect();
        self
    }

    /// Enable HTML GET form pages + POST routes under [`Self::mount`] (Laravel-style paths).
    pub fn web_forms(mut self, yes: bool) -> Self {
        self.web_forms = yes;
        self
    }

    pub fn mount(mut self, path: impl Into<String>) -> Self {
        self.mount = path.into();
        self
    }

    pub fn api_mount(mut self, path: impl Into<String>) -> Self {
        self.api_mount = Some(path.into());
        self
    }

    pub fn no_api(mut self) -> Self {
        self.api_mount = None;
        self
    }

    pub fn paths(mut self, paths: FortifyPaths) -> Self {
        self.paths = paths;
        self
    }

    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = secret.into();
        self
    }

    pub fn public_url(mut self, url: impl Into<String>) -> Self {
        self.public_url = url.into();
        self
    }

    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = name.into();
        self
    }

    pub fn home(mut self, path: impl Into<String>) -> Self {
        self.home_path = path.into();
        self
    }

    pub fn login_path(mut self, path: impl Into<String>) -> Self {
        self.login_path = path.into();
        self
    }

    /// Alias for [`Self::login_path`] (HTML guard redirect).
    pub fn login_redirect(mut self, path: impl Into<String>) -> Self {
        self.login_path = path.into();
        self
    }

    pub fn profile_path(mut self, path: impl Into<String>) -> Self {
        self.profile_path = path.into();
        self
    }

    pub fn verify_path(mut self, path: impl Into<String>) -> Self {
        self.verify_path = path.into();
        self
    }

    pub fn confirm_password_path(mut self, path: impl Into<String>) -> Self {
        self.confirm_password_path = path.into();
        self
    }

    pub fn two_factor_challenge_path(mut self, path: impl Into<String>) -> Self {
        self.two_factor_challenge_path = path.into();
        self
    }

    pub fn allow_system_role_delete(mut self, yes: bool) -> Self {
        self.allow_system_role_delete = yes;
        self
    }

    pub fn after_register<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(CurrentUser, Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Request>> + Send + 'static,
    {
        self.after_register =
            Some(Arc::new(move |u, r| Box::pin(f(u, r)) as BoxFuture<_>) as AfterRegisterFn);
        self
    }

    /// Require authenticated [`CurrentUser`] (redirect uses `FortifyState.login_path`).
    pub fn guard() -> MwEntry {
        fortify_guard_from_state()
    }

    pub fn guard_to(login: impl Into<String>) -> MwEntry {
        fortify_guard(login)
    }

    /// Require verified email (path from state).
    pub fn verified() -> MwEntry {
        guard::verified_from_state()
    }

    pub fn verified_to(path: impl Into<String>) -> MwEntry {
        guard::verified(path)
    }

    /// Require recent password confirmation (path from state).
    pub fn password_confirmed() -> MwEntry {
        guard::password_confirmed_from_state()
    }

    pub fn password_confirmed_to(path: impl Into<String>) -> MwEntry {
        guard::password_confirmed(path)
    }

    pub fn permission(slug: impl Into<String>) -> MwEntry {
        guard::permission(slug)
    }

    pub fn role(slug: impl Into<String>) -> MwEntry {
        guard::role(slug)
    }
}

impl Default for Fortify {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Fortify {
    fn id(&self) -> &'static str {
        "fortify"
    }

    fn requires(&self) -> &'static [&'static str] {
        let needs_mail = self.features.contains(&Feature::EmailVerification)
            || self.features.contains(&Feature::ResetPasswords);
        if needs_mail {
            &["db", "session", "mail"]
        } else {
            &["db", "session"]
        }
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Fortify")
            .description("Register/login, verify, reset, 2FA, profile, roles")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        if self.secret.is_empty() {
            app.on_startup(|_s| async {
                Err(Error::Internal("FORTIFY_SECRET / APP_KEY is empty".into()))
            });
            return;
        }

        let state = FortifyState {
            features: self.features.clone(),
            secret: self.secret.clone(),
            public_url: self.public_url.clone(),
            app_name: self.app_name.clone(),
            home_path: self.home_path.clone(),
            login_path: self.login_path.clone(),
            profile_path: self.profile_path.clone(),
            verify_path: self.verify_path.clone(),
            confirm_password_path: self.confirm_password_path.clone(),
            two_factor_challenge_path: self.two_factor_challenge_path.clone(),
            two_factor_path: self.two_factor_path.clone(),
            paths: self.paths.clone(),
            allow_system_role_delete: self.allow_system_role_delete,
            after_register: self.after_register.clone(),
        };
        app.state(state);

        app.install(
            Passport::new()
                .serialize_user(|req| {
                    let id = req
                        .get::<CurrentUser>()
                        .map(|u| u.id.to_string())
                        .or_else(|| {
                            req.get::<sova_passport::Authenticated>()
                                .map(|a| a.id.clone())
                        });
                    async move { Ok(id) }
                })
                .deserialize_user(|id, mut req| async move {
                    let Ok(uid) = id.parse::<i64>() else {
                        return Ok(req);
                    };
                    let db = req.db().clone();
                    if let Some(cu) = store::load_current_user(&db, uid).await? {
                        req.set(RateLimitIdentity(cu.id.to_string()));
                        req.set(cu);
                    }
                    Ok(req)
                }),
        );

        if self.web_forms {
            let mut web = Router::new();
            mount_web(&mut web, &self.features);
            let mount = if self.mount == "/" {
                "".to_string()
            } else {
                self.mount.trim_end_matches('/').to_string()
            };
            if mount.is_empty() {
                app.mount("", web);
            } else {
                app.mount(&mount, web);
            }
        }

        if let Some(api) = self.api_mount {
            let mut r = Router::new();
            mount_api(&mut r, &self.features, &self.paths);
            app.mount(&api, r);
        }
    }
}

fn mount_web(web: &mut Router, features: &HashSet<Feature>) {
    if features.contains(&Feature::Registration) {
        web.get("/register", actions::register_form);
        web.post("/register", actions::register);
        #[cfg(feature = "vld")]
        web.validate_form::<crate::forms::RegisterForm>();
    }

    web.get("/login", actions::login_form);
    web.post("/login", actions::login);
    web.route_middleware(RateLimit::login().middleware());
    #[cfg(feature = "vld")]
    web.validate_form::<crate::forms::LoginForm>();

    web.post("/logout", actions::logout);

    if features.contains(&Feature::ResetPasswords) {
        web.post("/forgot-password", actions::forgot_password);
        web.route_middleware(RateLimit::forgot().middleware());
        #[cfg(feature = "vld")]
        web.validate_form::<crate::forms::ForgotForm>();

        web.post("/reset-password", actions::reset_password);
        #[cfg(feature = "vld")]
        web.validate_form::<crate::forms::ResetForm>();
    }
    if features.contains(&Feature::EmailVerification) {
        web.post("/email/verify", actions::verify_email);
        web.post(
            "/email/verification-notification",
            actions::resend_verification,
        );
        web.route_middleware(RateLimit::resend().middleware());
    }
    if features.contains(&Feature::UpdateProfile) {
        web.get("/user/profile", actions::profile_get);
        web.post("/user/profile", actions::update_profile);
        #[cfg(feature = "vld")]
        web.validate_form::<crate::forms::ProfileForm>();
    }
    if features.contains(&Feature::UpdatePasswords) {
        web.post("/user/password", actions::update_password);
        #[cfg(feature = "vld")]
        web.validate_form::<crate::forms::PasswordForm>();
    }

    web.post("/user/confirm-password", actions::confirm_password);
    #[cfg(feature = "vld")]
    web.validate_form::<crate::forms::ConfirmPasswordForm>();
    web.get(
        "/user/confirmed-password-status",
        actions::confirmed_password_status,
    );

    if features.contains(&Feature::TwoFactor) {
        web.post(
            "/user/two-factor-authentication",
            actions::two_factor_enable,
        );

        web.post(
            "/user/confirmed-two-factor-authentication",
            actions::two_factor_confirm,
        );
        #[cfg(feature = "vld")]
        web.validate_form::<crate::forms::TwoFactorCodeForm>();

        web.post(
            "/user/two-factor-authentication/disable",
            actions::two_factor_disable,
        );
        #[cfg(feature = "vld")]
        web.validate_form::<crate::forms::DisableTwoFactorForm>();

        web.delete(
            "/user/two-factor-authentication",
            actions::two_factor_disable,
        );
        #[cfg(feature = "vld")]
        web.validate_form::<crate::forms::DisableTwoFactorForm>();

        web.get("/user/two-factor-qr-code", actions::two_factor_qr_code);
        web.get(
            "/user/two-factor-secret-key",
            actions::two_factor_secret_key,
        );
        web.get(
            "/user/two-factor-recovery-codes",
            actions::two_factor_recovery_codes_get,
        );
        web.post(
            "/user/two-factor-recovery-codes",
            actions::two_factor_recovery_codes_regen,
        );

        web.post("/two-factor-challenge", actions::two_factor_challenge);
        web.route_middleware(RateLimit::challenge().middleware());
        #[cfg(feature = "vld")]
        web.validate_form::<crate::forms::TwoFactorCodeForm>();
    }
}

fn mount_api(r: &mut Router, features: &HashSet<Feature>, paths: &FortifyPaths) {
    if features.contains(&Feature::Registration) {
        r.post(&paths.register, actions::register);
        #[cfg(feature = "vld")]
        r.validate_body::<crate::forms::RegisterForm>();
    }
    r.post(&paths.login, actions::login);
    r.route_middleware(RateLimit::login().middleware());
    #[cfg(feature = "vld")]
    r.validate_body::<crate::forms::LoginForm>();

    r.post(&paths.logout, actions::logout);
    r.get(&paths.me, actions::me);
    r.get(&paths.profile, actions::me);

    if features.contains(&Feature::ResetPasswords) {
        r.post(&paths.forgot_password, actions::forgot_password);
        r.route_middleware(RateLimit::forgot().middleware());
        #[cfg(feature = "vld")]
        r.validate_body::<crate::forms::ForgotForm>();
        r.post(&paths.reset_password, actions::reset_password);
        #[cfg(feature = "vld")]
        r.validate_body::<crate::forms::ResetForm>();
    }
    if features.contains(&Feature::EmailVerification) {
        r.post(&paths.verify_email, actions::verify_email);
        r.post(&paths.resend_verification, actions::resend_verification);
        r.route_middleware(RateLimit::resend().middleware());
    }
    if features.contains(&Feature::UpdateProfile) {
        r.post(&paths.profile, actions::update_profile);
        #[cfg(feature = "vld")]
        r.validate_body::<crate::forms::ProfileForm>();
    }
    if features.contains(&Feature::UpdatePasswords) {
        r.post(&paths.password, actions::update_password);
        #[cfg(feature = "vld")]
        r.validate_body::<crate::forms::PasswordForm>();
    }
    r.post(&paths.confirm_password, actions::confirm_password);
    #[cfg(feature = "vld")]
    r.validate_body::<crate::forms::ConfirmPasswordForm>();
    r.get(
        &paths.confirmed_password_status,
        actions::confirmed_password_status,
    );

    if features.contains(&Feature::TwoFactor) {
        r.post(&paths.two_factor, actions::two_factor_enable);
        r.post(&paths.two_factor_confirm, actions::two_factor_confirm);
        #[cfg(feature = "vld")]
        r.validate_body::<crate::forms::TwoFactorCodeForm>();
        r.post(&paths.two_factor_disable, actions::two_factor_disable);
        #[cfg(feature = "vld")]
        r.validate_body::<crate::forms::DisableTwoFactorForm>();
        r.delete(&paths.two_factor, actions::two_factor_disable);
        #[cfg(feature = "vld")]
        r.validate_body::<crate::forms::DisableTwoFactorForm>();
        r.get(&paths.two_factor_qr, actions::two_factor_qr_code);
        r.get(&paths.two_factor_secret, actions::two_factor_secret_key);
        r.get(
            &paths.two_factor_recovery,
            actions::two_factor_recovery_codes_get,
        );
        r.post(
            &paths.two_factor_recovery,
            actions::two_factor_recovery_codes_regen,
        );
        r.post(&paths.two_factor_challenge, actions::two_factor_challenge);
        r.route_middleware(RateLimit::challenge().middleware());
        #[cfg(feature = "vld")]
        r.validate_body::<crate::forms::TwoFactorCodeForm>();
    }
    if features.contains(&Feature::Roles) {
        mount_rbac(r, paths);
    }
}

fn mount_rbac(r: &mut Router, paths: &FortifyPaths) {
    let roles = paths.roles.trim_end_matches('/').to_string();
    let perms = paths.permissions.trim_end_matches('/').to_string();
    let users = paths.users.trim_end_matches('/').to_string();

    let role_id = format!("{roles}/:id");
    let role_perms = format!("{roles}/:id/permissions");
    let perm_id = format!("{perms}/:id");
    let user_roles = format!("{users}/:id/roles");

    r.get(&roles, actions::roles_list);
    r.post(&roles, actions::roles_create);
    r.get(&role_id, actions::roles_show);
    r.put(&role_id, actions::roles_update);
    r.patch(&role_id, actions::roles_update);
    r.delete(&role_id, actions::roles_delete);
    r.put(&role_perms, actions::roles_sync_permissions);

    r.get(&perms, actions::permissions_list);
    r.post(&perms, actions::permissions_create);
    r.put(&perm_id, actions::permissions_update);
    r.patch(&perm_id, actions::permissions_update);
    r.delete(&perm_id, actions::permissions_delete);

    r.get(&user_roles, actions::user_roles_list);
    r.put(&user_roles, actions::user_roles_sync);
}
