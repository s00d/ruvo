//! HS256 JWT helpers (feature `jwt`).

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Standard access-token claims used by [`Jwt::issue_access`] / [`Jwt::decode_access`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    /// Subject (usually user id as string).
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    /// Unique token id (for logging / future revoke lists).
    pub jti: String,
}

/// Thin HS256 wrapper around `jsonwebtoken`.
#[derive(Clone)]
pub struct Jwt {
    secret: String,
}

impl Jwt {
    pub fn hs256(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
        }
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn encode<C: Serialize>(&self, claims: &C) -> Result<String, JwtError> {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| JwtError(e.to_string()))
    }

    pub fn decode<C: DeserializeOwned>(&self, token: &str) -> Result<C, JwtError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        decode::<C>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map(|d| d.claims)
        .map_err(|e| JwtError(e.to_string()))
    }

    /// Issue an access JWT for `sub` with TTL in seconds.
    pub fn issue_access(&self, sub: impl Into<String>, ttl_secs: u64) -> Result<String, JwtError> {
        let now = now_secs();
        let claims = Claims {
            sub: sub.into(),
            iat: now,
            exp: now.saturating_add(ttl_secs),
            jti: random_jti(),
        };
        self.encode(&claims)
    }

    pub fn decode_access(&self, token: &str) -> Result<Claims, JwtError> {
        self.decode(token)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn random_jti() -> String {
    let mut buf = [0u8; 16];
    let _ = getrandom::getrandom(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Clone)]
pub struct JwtError(pub String);

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "jwt: {}", self.0)
    }
}

impl std::error::Error for JwtError {}
