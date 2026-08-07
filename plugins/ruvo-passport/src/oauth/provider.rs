//! OAuth2 provider config (GitHub / Google / custom).

use ruvo_core::{Error, Result};

/// Built-in or custom OAuth2 provider.
#[derive(Clone, Debug)]
pub struct OauthProvider {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub scopes: Vec<String>,
    /// Override redirect URI; otherwise `{public_url}{mount}/{name}/callback`.
    pub redirect_uri: Option<String>,
    /// How to parse userinfo JSON into [`super::OauthProfile`].
    pub profile_kind: ProfileKind,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Github,
    Google,
    /// `id` field + optional `email` / `name` at top level.
    Generic,
}

impl OauthProvider {
    pub fn github() -> Self {
        Self {
            name: "github".into(),
            client_id: String::new(),
            client_secret: String::new(),
            authorization_url: "https://github.com/login/oauth/authorize".into(),
            token_url: "https://github.com/login/oauth/access_token".into(),
            userinfo_url: "https://api.github.com/user".into(),
            scopes: vec!["read:user".into(), "user:email".into()],
            redirect_uri: None,
            profile_kind: ProfileKind::Github,
        }
    }

    pub fn google() -> Self {
        Self {
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
        }
    }

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

    /// `{NAME}_CLIENT_ID` / `{NAME}_CLIENT_SECRET` / optional `{NAME}_REDIRECT_URI`
    /// (NAME = uppercased provider name, e.g. `GITHUB_CLIENT_ID`).
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
        if self.client_id.is_empty() || self.client_secret.is_empty() {
            return Err(Error::Internal(format!(
                "oauth provider `{}`: missing client_id/client_secret",
                self.name
            )));
        }
        Ok(())
    }
}
