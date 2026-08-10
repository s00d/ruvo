//! Signed one-time tokens (email verify) + recovery code helpers.

use crate::store::{hash_token, random_token};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sova_core::{Error, Result};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn sign_payload(secret: &str, payload: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key length");
    mac.update(payload.as_bytes());
    hex_encode(mac.finalize().into_bytes())
}

pub fn verify_payload(secret: &str, payload: &str, sig: &str) -> bool {
    let expected = sign_payload(secret, payload);
    constant_eq(expected.as_bytes(), sig.as_bytes())
}

fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// `user_id.exp.sig` for email verification (24h).
pub fn make_verify_token(secret: &str, user_id: i64) -> String {
    let exp = now_secs() + 86_400;
    let payload = format!("verify:{user_id}:{exp}");
    let sig = sign_payload(secret, &payload);
    format!("{user_id}.{exp}.{sig}")
}

pub fn parse_verify_token(secret: &str, token: &str) -> Result<i64> {
    let mut parts = token.split('.');
    let id: i64 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::BadRequest("invalid verification token".into()))?;
    let exp: u64 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::BadRequest("invalid verification token".into()))?;
    let sig = parts
        .next()
        .ok_or_else(|| Error::BadRequest("invalid verification token".into()))?;
    if parts.next().is_some() {
        return Err(Error::BadRequest("invalid verification token".into()));
    }
    if exp < now_secs() {
        return Err(Error::BadRequest("verification token expired".into()));
    }
    let payload = format!("verify:{id}:{exp}");
    if !verify_payload(secret, &payload, sig) {
        return Err(Error::BadRequest("invalid verification token".into()));
    }
    Ok(id)
}

pub fn generate_recovery_codes(n: usize) -> Vec<String> {
    (0..n)
        .map(|_| {
            let t = random_token();
            format!("{}-{}", &t[..4], &t[4..8])
        })
        .collect()
}

pub fn hash_recovery_codes(codes: &[String]) -> Result<String> {
    let hashed: Vec<String> = codes.iter().map(|c| hash_token(c)).collect();
    serde_json::to_string(&hashed).map_err(|e| Error::Internal(e.to_string()))
}

pub fn consume_recovery_code(codes_json: &str, raw: &str) -> Result<Option<String>> {
    let mut hashed: Vec<String> =
        serde_json::from_str(codes_json).map_err(|e| Error::Internal(e.to_string()))?;
    let want = hash_token(raw.trim());
    let Some(pos) = hashed.iter().position(|h| h == &want) else {
        return Ok(None);
    };
    hashed.remove(pos);
    Ok(Some(
        serde_json::to_string(&hashed).map_err(|e| Error::Internal(e.to_string()))?,
    ))
}
