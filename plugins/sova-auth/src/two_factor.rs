//! TOTP helpers.

use sova_core::{Error, Result};
use totp_rs::{Algorithm, Secret, TOTP};

pub fn generate_secret() -> String {
    match Secret::generate_secret().to_encoded() {
        Secret::Encoded(s) => s,
        Secret::Raw(bytes) => Secret::Raw(bytes).to_encoded().to_string(),
    }
}

pub fn make_totp(secret_b32: &str, issuer: &str, account: &str) -> Result<TOTP> {
    let secret = Secret::Encoded(secret_b32.to_string())
        .to_bytes()
        .map_err(|e| Error::Internal(format!("totp secret: {e}")))?;
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some(issuer.to_string()),
        account.to_string(),
    )
    .map_err(|e| Error::Internal(format!("totp: {e}")))
}

pub fn verify_code(secret_b32: &str, code: &str) -> Result<bool> {
    let totp = make_totp(secret_b32, "Sova", "user")?;
    Ok(totp.check_current(code.trim()).unwrap_or(false))
}

pub fn otpauth_url(secret_b32: &str, issuer: &str, account: &str) -> Result<String> {
    Ok(make_totp(secret_b32, issuer, account)?.get_url())
}
