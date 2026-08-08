//! Apple Sign In driver (authorization code + `id_token`, form_post).

use super::{impl_driver_from, Driver};
use crate::oauth::provider::{OauthProvider, ProfileKind};
use ruvo_core::{Error, Result};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

/// Apple Sign In driver.
#[derive(Clone, Debug)]
pub struct Apple(OauthProvider);

impl Apple {
    pub fn new() -> Self {
        Self(OauthProvider {
            name: "apple".into(),
            client_id: String::new(),
            client_secret: String::new(),
            authorization_url: "https://appleid.apple.com/auth/authorize".into(),
            token_url: "https://appleid.apple.com/auth/token".into(),
            userinfo_url: String::new(),
            scopes: vec!["name".into(), "email".into()],
            redirect_uri: None,
            profile_kind: ProfileKind::Apple,
            auth_params: vec![("response_mode".into(), "form_post".into())],
            team_id: None,
            key_id: None,
            private_key_pem: None,
        })
    }

    pub fn team_id(self, id: impl Into<String>) -> Self {
        Self(self.0.team_id(id))
    }

    pub fn key_id(self, id: impl Into<String>) -> Self {
        Self(self.0.key_id(id))
    }

    pub fn private_key_pem(self, pem: impl Into<String>) -> Self {
        Self(self.0.private_key_pem(pem))
    }

    /// `{NAME}_*` plus `APPLE_TEAM_ID` / `APPLE_KEY_ID` / `APPLE_PRIVATE_KEY`.
    pub fn from_env(self) -> Self {
        let mut p = self.0.from_env();
        apply_env(&mut p);
        Self(p)
    }
}

impl Default for Apple {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver for Apple {
    fn into_provider(self) -> OauthProvider {
        self.0
    }

    fn from_provider(provider: OauthProvider) -> Self {
        Self(provider)
    }

    /// Prefer [`Apple::from_env`] so Apple key material is loaded too.
    fn from_env(self) -> Self {
        Apple::from_env(self)
    }
}

impl_driver_from!(Apple);

fn apply_env(p: &mut OauthProvider) {
    if let Ok(v) = std::env::var("APPLE_TEAM_ID") {
        p.team_id = Some(v);
    }
    if let Ok(v) = std::env::var("APPLE_KEY_ID") {
        p.key_id = Some(v);
    }
    if let Ok(v) = std::env::var("APPLE_PRIVATE_KEY") {
        p.private_key_pem = Some(v);
    }
}

#[derive(Serialize)]
struct AppleSecretClaims {
    iss: String,
    iat: u64,
    exp: u64,
    aud: String,
    sub: String,
}

/// Mint ES256 client_secret JWT from Team ID + Key ID + `.p8` PEM.
pub(crate) fn mint_client_secret(provider: &OauthProvider) -> Result<String> {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

    let team_id = provider.team_id.as_deref().unwrap_or_default();
    let key_id = provider.key_id.as_deref().unwrap_or_default();
    let pem = provider.private_key_pem.as_deref().unwrap_or_default();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Apple allows up to 6 months.
    let exp = now + 60 * 60 * 24 * 180;
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(key_id.to_string());
    let claims = AppleSecretClaims {
        iss: team_id.to_string(),
        iat: now,
        exp,
        aud: "https://appleid.apple.com".into(),
        sub: provider.client_id.clone(),
    };
    let key = EncodingKey::from_ec_pem(pem.as_bytes())
        .map_err(|e| Error::Internal(format!("apple private key: {e}")))?;
    encode(&header, &claims, &key)
        .map_err(|e| Error::Internal(format!("apple client_secret jwt: {e}")))
}
