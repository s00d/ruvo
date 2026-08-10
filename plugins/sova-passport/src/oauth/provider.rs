//! OAuth2 provider config bag + builders.
//!
//! Prefer typed drivers in [`super::drivers`] (`Github`, `Google`, `Apple`, `Custom`).
//! Shortcuts [`OauthProvider::github`] etc. still return a finished [`OauthProvider`].

use super::drivers::{self, Apple, Driver, Github, Google};
use sova_core::{Error, Result};

/// Built-in or custom OAuth2 provider (output of a driver `.build()` / `Into`).
#[derive(Clone, Debug)]
pub struct OauthProvider {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    /// Empty → profile from `id_token` (Apple / OIDC).
    pub userinfo_url: String,
    pub scopes: Vec<String>,
    /// Override redirect URI; otherwise `{public_url}{mount}/{name}/callback`.
    pub redirect_uri: Option<String>,
    /// How to parse userinfo / id_token JSON into [`super::OauthProfile`].
    pub profile_kind: ProfileKind,
    /// Extra authorize query params (e.g. `access_type=offline`, `response_mode=form_post`).
    pub auth_params: Vec<(String, String)>,
    /// Apple Sign In: Team ID (iss of client_secret JWT).
    pub team_id: Option<String>,
    /// Apple Sign In: Key ID (`kid` header).
    pub key_id: Option<String>,
    /// Apple Sign In: `.p8` private key PEM (ES256).
    pub private_key_pem: Option<String>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Github,
    Google,
    /// Claims from Apple / OIDC `id_token` (`sub`, optional `email`).
    Apple,
    /// `id` / `sub` field + optional `email` / `name` at top level.
    Generic,
}

impl OauthProvider {
    /// [`Github::new`](super::drivers::Github::new) → provider.
    pub fn github() -> Self {
        Github::new().build()
    }

    /// [`Google::new`](super::drivers::Google::new) → provider.
    pub fn google() -> Self {
        Google::new().build()
    }

    /// [`Apple::new`](super::drivers::Apple::new) → provider.
    pub fn apple() -> Self {
        Apple::new().build()
    }

    /// Custom / third-party IdP (Generic profile). Prefer [`super::drivers::Custom`].
    pub fn custom(
        name: impl Into<String>,
        authorization_url: impl Into<String>,
        token_url: impl Into<String>,
        userinfo_url: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            client_id: String::new(),
            client_secret: String::new(),
            authorization_url: authorization_url.into(),
            token_url: token_url.into(),
            userinfo_url: userinfo_url.into(),
            scopes: Vec::new(),
            redirect_uri: None,
            profile_kind: ProfileKind::Generic,
            auth_params: Vec::new(),
            team_id: None,
            key_id: None,
            private_key_pem: None,
        }
    }

    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        self.client_id = id.into();
        self
    }

    pub fn client_secret(mut self, secret: impl Into<String>) -> Self {
        self.client_secret = secret.into();
        self
    }

    pub fn scopes(mut self, scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    pub fn redirect_uri(mut self, uri: impl Into<String>) -> Self {
        self.redirect_uri = Some(uri.into());
        self
    }

    pub fn userinfo_url(mut self, url: impl Into<String>) -> Self {
        self.userinfo_url = url.into();
        self
    }

    pub fn profile_kind(mut self, kind: ProfileKind) -> Self {
        self.profile_kind = kind;
        self
    }

    pub fn auth_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.auth_params.push((key.into(), value.into()));
        self
    }

    pub fn auth_params(
        mut self,
        params: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.auth_params = params
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    pub fn team_id(mut self, id: impl Into<String>) -> Self {
        self.team_id = Some(id.into());
        self
    }

    pub fn key_id(mut self, id: impl Into<String>) -> Self {
        self.key_id = Some(id.into());
        self
    }

    pub fn private_key_pem(mut self, pem: impl Into<String>) -> Self {
        self.private_key_pem = Some(pem.into());
        self
    }

    /// `{NAME}_CLIENT_ID` / `{NAME}_CLIENT_SECRET` / optional `{NAME}_REDIRECT_URI`
    /// (NAME = uppercased provider name, e.g. `GITHUB_CLIENT_ID`).
    ///
    /// For Apple key material use [`Apple::from_env`](super::drivers::Apple::from_env).
    pub fn from_env(mut self) -> Self {
        let prefix = self.name.to_uppercase().replace('-', "_");
        if let Ok(v) = std::env::var(format!("{prefix}_CLIENT_ID")) {
            self.client_id = v;
        }
        if let Ok(v) = std::env::var(format!("{prefix}_CLIENT_SECRET")) {
            self.client_secret = v;
        }
        if let Ok(v) = std::env::var(format!("{prefix}_REDIRECT_URI")) {
            self.redirect_uri = Some(v);
        }
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.client_id.is_empty() {
            return Err(Error::Internal(format!(
                "oauth provider `{}`: missing client_id",
                self.name
            )));
        }
        if !self.client_secret.is_empty() {
            return Ok(());
        }
        if self.has_apple_key() {
            return Ok(());
        }
        Err(Error::Internal(format!(
            "oauth provider `{}`: missing client_id/client_secret",
            self.name
        )))
    }

    fn has_apple_key(&self) -> bool {
        self.team_id.as_ref().is_some_and(|s| !s.is_empty())
            && self.key_id.as_ref().is_some_and(|s| !s.is_empty())
            && self.private_key_pem.as_ref().is_some_and(|s| !s.is_empty())
    }

    /// Static secret, or mint Apple client_secret JWT from `.p8` key.
    pub fn resolve_client_secret(&self) -> Result<String> {
        if !self.client_secret.is_empty() {
            return Ok(self.client_secret.clone());
        }
        if self.has_apple_key() {
            return drivers::mint_apple_secret(self);
        }
        Err(Error::Internal(format!(
            "oauth provider `{}`: no client_secret",
            self.name
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::drivers::{Apple, Driver, Github, Google};

    #[test]
    fn google_has_offline_params() {
        let g = Google::new().build();
        assert!(g
            .auth_params
            .iter()
            .any(|(k, v)| k == "access_type" && v == "offline"));
    }

    #[test]
    fn apple_validate_with_key_material() {
        let p = Apple::new()
            .client_id("com.example.app")
            .team_id("TEAM")
            .key_id("KEY")
            .private_key_pem("-----BEGIN PRIVATE KEY-----\nX\n-----END PRIVATE KEY-----")
            .build();
        assert!(p.validate().is_ok());
    }

    #[test]
    fn apple_validate_rejects_empty() {
        assert!(Apple::new().build().validate().is_err());
    }

    #[test]
    fn drivers_into_provider() {
        let p: OauthProvider = Github::new().into();
        assert_eq!(p.name, "github");
        assert_eq!(Google::new().build().name, OauthProvider::google().name);
        assert_eq!(Apple::new().build().profile_kind, ProfileKind::Apple);
    }

    #[test]
    fn driver_common_methods() {
        let p = Github::new()
            .client_id("id")
            .client_secret("sec")
            .scopes(["a", "b"])
            .auth_param("foo", "bar")
            .build();
        assert_eq!(p.client_id, "id");
        assert_eq!(p.scopes, vec!["a", "b"]);
        assert!(p.auth_params.iter().any(|(k, v)| k == "foo" && v == "bar"));
    }
}
