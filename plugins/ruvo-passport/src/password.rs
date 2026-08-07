//! Argon2 password helpers (feature `jwt`).

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
use ruvo_core::{Error, Result};

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| Error::Internal(format!("password hash: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed =
        PasswordHash::new(hash).map_err(|e| Error::Internal(format!("bad password hash: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}
