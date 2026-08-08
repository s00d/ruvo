//! Custom / third-party OAuth2 driver (Generic profile).

use super::{impl_driver_from, Driver};
use crate::oauth::provider::{OauthProvider, ProfileKind};

/// Bring-your-own IdP — same builder methods as built-ins.
#[derive(Clone, Debug)]
pub struct Custom(OauthProvider);

impl Custom {
    pub fn new(
        name: impl Into<String>,
        authorization_url: impl Into<String>,
        token_url: impl Into<String>,
        userinfo_url: impl Into<String>,
    ) -> Self {
        Self(OauthProvider::custom(
            name,
            authorization_url,
            token_url,
            userinfo_url,
        ))
    }

    pub fn profile_kind(self, kind: ProfileKind) -> Self {
        Self(self.0.profile_kind(kind))
    }
}

impl Driver for Custom {
    fn into_provider(self) -> OauthProvider {
        self.0
    }

    fn from_provider(provider: OauthProvider) -> Self {
        Self(provider)
    }
}

impl_driver_from!(Custom);
