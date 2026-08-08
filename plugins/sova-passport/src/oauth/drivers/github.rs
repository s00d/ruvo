//! GitHub OAuth2 driver.

use super::{impl_driver_from, Driver};
use crate::oauth::provider::{OauthProvider, ProfileKind};

/// GitHub authorization-code driver.
#[derive(Clone, Debug)]
pub struct Github(OauthProvider);

impl Github {
    pub fn new() -> Self {
        Self(OauthProvider {
            name: "github".into(),
            client_id: String::new(),
            client_secret: String::new(),
            authorization_url: "https://github.com/login/oauth/authorize".into(),
            token_url: "https://github.com/login/oauth/access_token".into(),
            userinfo_url: "https://api.github.com/user".into(),
            scopes: vec!["read:user".into(), "user:email".into()],
            redirect_uri: None,
            profile_kind: ProfileKind::Github,
            auth_params: Vec::new(),
            team_id: None,
            key_id: None,
            private_key_pem: None,
        })
    }
}

impl Default for Github {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver for Github {
    fn into_provider(self) -> OauthProvider {
        self.0
    }

    fn from_provider(provider: OauthProvider) -> Self {
        Self(provider)
    }
}

impl_driver_from!(Github);
