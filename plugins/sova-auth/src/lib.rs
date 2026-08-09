//! Fortify-style authentication for Sova (register, verify, reset, 2FA, RBAC).
//!
//! Builds on [`sova_passport`] (session login) + [`sova_db`]. Enable feature `mail`
//! (and install [`sova_mail::Mail`]) for email verification / password reset.
//!
//! ```ignore
//! app.install(Db::from_env().migrations::<sova_auth::AuthMigrator>());
//! app.install(memory_sessions());
//! app.install(
//!   Fortify::new()
//!     .features([Feature::Registration, Feature::ResetPasswords, /* … */])
//!     .api_mount("/api/auth")
//!     .login_redirect("/login")
//!     .home("/cabinet"),
//! );
//! // Mail only when ResetPasswords / EmailVerification:
//! // app.install(Mail::from_env());
//! cabinet.use_middleware(Fortify::guard());
//!
//! // Programmatic login (impersonation / seed / admin switch):
//! let cu = load_current_user(db, id).await?.unwrap();
//! req.login_user(cu);   // regenerates session + passport:user
//! req.logout_user();
//! ```

mod actions;
mod feature;
mod forms;
mod guard;
#[cfg(feature = "mail")]
mod mail;
mod migration;
mod paths;
mod plugin;
mod state;
mod store;
mod token;
mod two_factor;

#[cfg(feature = "activity")]
mod activity_log;

pub mod entity;
pub mod testing;

#[cfg(feature = "mail")]
pub use mail::{send_reset, send_verify, ResetPasswordMail, VerifyEmailMail};
pub use feature::Feature;
pub use guard::AuthExt;
pub use migration::AuthMigrator;
pub use paths::FortifyPaths;
pub use plugin::Fortify;
pub use store::{
    assign_role, create_permission, create_role, delete_permission, delete_role, find_user_by_email,
    find_user_by_id, list_permissions, list_roles, load_current_user, mark_email_verified,
    register_user, revoke_role, set_avatar, set_user_roles, sync_role_permissions, update_permission,
    update_role, user_ids_with_permission, user_ids_with_role, CurrentUser,
};
pub use token::{make_verify_token, parse_verify_token};

#[cfg(feature = "vld")]
pub use forms::{
    ConfirmPasswordForm, DisableTwoFactorForm, ForgotForm, LoginForm, PasswordForm, ProfileForm,
    RegisterForm, ResetForm, TwoFactorCodeForm,
};
