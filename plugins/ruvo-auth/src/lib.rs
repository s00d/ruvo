//! Fortify-style authentication for Ruvo (register, verify, reset, 2FA, RBAC).
//!
//! Builds on [`ruvo_passport`] (session login) + [`ruvo_mail`] + [`ruvo_db`].
//!
//! ```ignore
//! app.install(Db::from_env().migrations::<ruvo_auth::AuthMigrator>());
//! app.install(Mail::from_env());
//! app.install(memory_sessions());
//! app.install(
//!   Fortify::new()
//!     .features([Feature::Registration, Feature::ResetPasswords, /* … */])
//!     .api_mount("/api/auth")
//!     .login_redirect("/login")
//!     .home("/cabinet"),
//! );
//! cabinet.use_middleware(Fortify::guard());
//! ```

mod actions;
mod feature;
mod forms;
mod guard;
mod limiter;
mod mail;
mod migration;
mod paths;
mod plugin;
mod state;
mod store;
mod token;
mod two_factor;

pub mod entity;

pub use feature::Feature;
pub use guard::AuthExt;
pub use migration::AuthMigrator;
pub use paths::FortifyPaths;
pub use plugin::Fortify;
pub use store::{
    assign_role, create_permission, create_role, delete_permission, delete_role, find_user_by_email,
    find_user_by_id, list_permissions, list_roles, load_current_user, mark_email_verified,
    register_user, revoke_role, set_avatar, set_user_roles, sync_role_permissions, update_permission,
    update_role, CurrentUser,
};
pub use token::{make_verify_token, parse_verify_token};

#[cfg(feature = "vld")]
pub use forms::{
    ConfirmPasswordForm, DisableTwoFactorForm, ForgotForm, LoginForm, PasswordForm, ProfileForm,
    RegisterForm, ResetForm, TwoFactorCodeForm,
};
