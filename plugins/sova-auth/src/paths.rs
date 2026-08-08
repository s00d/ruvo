//! Configurable Fortify API path segments (under `api_mount`).

/// Relative path segments for Fortify JSON routes.
#[derive(Clone, Debug)]
pub struct FortifyPaths {
    pub login: String,
    pub register: String,
    pub logout: String,
    pub me: String,
    pub profile: String,
    pub password: String,
    pub forgot_password: String,
    pub reset_password: String,
    pub verify_email: String,
    pub resend_verification: String,
    pub confirm_password: String,
    pub confirmed_password_status: String,
    pub two_factor: String,
    pub two_factor_confirm: String,
    pub two_factor_disable: String,
    pub two_factor_qr: String,
    pub two_factor_secret: String,
    pub two_factor_recovery: String,
    pub two_factor_challenge: String,
    pub roles: String,
    pub permissions: String,
    pub users: String,
}

impl Default for FortifyPaths {
    fn default() -> Self {
        Self {
            login: "/login".into(),
            register: "/register".into(),
            logout: "/logout".into(),
            me: "/me".into(),
            profile: "/profile".into(),
            password: "/password".into(),
            forgot_password: "/forgot-password".into(),
            reset_password: "/reset-password".into(),
            verify_email: "/email/verify".into(),
            resend_verification: "/email/verification-notification".into(),
            confirm_password: "/confirm-password".into(),
            confirmed_password_status: "/confirmed-password-status".into(),
            two_factor: "/two-factor".into(),
            two_factor_confirm: "/two-factor/confirm".into(),
            two_factor_disable: "/two-factor/disable".into(),
            two_factor_qr: "/two-factor/qr-code".into(),
            two_factor_secret: "/two-factor/secret-key".into(),
            two_factor_recovery: "/two-factor/recovery-codes".into(),
            two_factor_challenge: "/two-factor/challenge".into(),
            roles: "/roles".into(),
            permissions: "/permissions".into(),
            users: "/users".into(),
        }
    }
}

impl FortifyPaths {
    pub fn new() -> Self {
        Self::default()
    }
}
