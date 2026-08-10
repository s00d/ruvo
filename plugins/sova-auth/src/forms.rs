//! Fortify form DTOs (vld schemas when feature `vld` is on).

#[cfg(feature = "vld")]
mod schemas {
    use vld::schema::VldSchema;

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct RegisterForm {
            pub name: String => vld::string().min(2).max(80),
            pub email: String => vld::string().email(),
            pub password: String => vld::string().min(8).max(128),
            pub password_confirmation: Option<String> => vld::string().min(8).max(128).optional(),
            pub csrf: Option<String> => vld::string().optional(),
        }
    }

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct LoginForm {
            pub email: String => vld::string().email(),
            pub password: String => vld::string().min(1),
            pub csrf: Option<String> => vld::string().optional(),
        }
    }

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct ForgotForm {
            pub email: String => vld::string().email(),
            pub csrf: Option<String> => vld::string().optional(),
        }
    }

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct ResetForm {
            pub email: String => vld::string().email(),
            pub token: String => vld::string().min(8),
            pub password: String => vld::string().min(8).max(128),
            pub password_confirmation: String => vld::string().min(8).max(128),
            pub csrf: Option<String> => vld::string().optional(),
        }
    }

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct ProfileForm {
            pub name: String => vld::string().min(2).max(80),
            pub email: String => vld::string().email(),
            pub csrf: Option<String> => vld::string().optional(),
        }
    }

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct PasswordForm {
            pub current_password: String => vld::string().min(1),
            pub password: String => vld::string().min(8).max(128),
            pub password_confirmation: String => vld::string().min(8).max(128),
            pub csrf: Option<String> => vld::string().optional(),
        }
    }

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct ConfirmPasswordForm {
            pub password: String => vld::string().min(1),
            pub csrf: Option<String> => vld::string().optional(),
        }
    }

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct TwoFactorCodeForm {
            pub code: Option<String> => vld::string().optional(),
            pub recovery_code: Option<String> => vld::string().optional(),
            pub csrf: Option<String> => vld::string().optional(),
        }
    }

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct DisableTwoFactorForm {
            pub password: Option<String> => vld::string().optional(),
            pub current_password: Option<String> => vld::string().optional(),
            pub csrf: Option<String> => vld::string().optional(),
        }
    }
}

#[cfg(feature = "vld")]
pub use schemas::*;

#[cfg(feature = "vld")]
sova_vld::doc_schema!(
    RegisterForm,
    LoginForm,
    ForgotForm,
    ResetForm,
    ProfileForm,
    PasswordForm,
    ConfirmPasswordForm,
    TwoFactorCodeForm,
    DisableTwoFactorForm,
);
