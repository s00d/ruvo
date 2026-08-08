//! Google OAuth2 / OpenID Connect driver.

use super::{impl_driver_from, Driver};
use crate::oauth::provider::{OauthProvider, ProfileKind};

/// Google OIDC driver (`access_type=offline`, `prompt=consent` by default).
#[derive(Clone, Debug)]
pub struct Google(OauthProvider);

impl Google {
    pub fn new() -> Self {
        Self(OauthProvider {
            name: "google".into(),
            client_id: String::new(),
            client_secret: String::new(),
            authorization_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
            userinfo_url: "https://openidconnect.googleapis.com/v1/userinfo".into(),
            scopes: vec![
                "openid".into(),
                "email".into(),
                "profile".into(),
            ],
            redirect_uri: None,
            profile_kind: ProfileKind::Google,
            auth_params: vec![
                ("access_type".into(), "offline".into()),
                ("prompt".into(), "consent".into()),
            ],
            team_id: None,
            key_id: None,
            private_key_pem: None,
        })
    }
}

impl Default for Google {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver for Google {
    fn into_provider(self) -> OauthProvider {
        self.0
    }

    fn from_provider(provider: OauthProvider) -> Self {
        Self(provider)
    }
}

impl_driver_from!(Google);
