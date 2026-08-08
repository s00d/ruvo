//! PKCE + signed state + token/userinfo HTTP.

use super::provider::{OauthProvider, ProfileKind};
use super::{OauthProfile, OauthTokens};
use base64::engine::general_purpose::{URL_SAFE_NO_PAD, STANDARD};
use base64::Engine;
use hmac::{Hmac, Mac};
use sova_core::{Error, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug)]
pub struct FlowState {
    pub provider: String,
    pub code_verifier: String,
    pub nonce: String,
    pub exp: u64,
}

pub fn random_urlsafe(len: usize) -> String {
    let mut buf = vec![0u8; len];
    let _ = getrandom::getrandom(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

pub fn pkce_challenge(verifier: &str) -> String {
    let mut h = Sha256::new();
    h.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(h.finalize())
}

pub fn sign_state(secret: &str, state: &FlowState) -> Result<String> {
    let payload = format!(
        "{}|{}|{}|{}",
        state.provider, state.code_verifier, state.nonce, state.exp
    );
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| Error::Internal(format!("hmac: {e}")))?;
    mac.update(payload.as_bytes());
    let sig = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(payload.as_bytes()),
        sig
    ))
}

pub fn verify_state(secret: &str, token: &str) -> Result<FlowState> {
    let (payload_b64, sig) = token
        .split_once('.')
        .ok_or_else(|| Error::BadRequest("invalid oauth state".into()))?;
    let payload = String::from_utf8(
        URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|_| Error::BadRequest("invalid oauth state".into()))?,
    )
    .map_err(|_| Error::BadRequest("invalid oauth state".into()))?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| Error::Internal(format!("hmac: {e}")))?;
    mac.update(payload.as_bytes());
    let expected = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    if !constant_time_eq(expected.as_bytes(), sig.as_bytes()) {
        return Err(Error::BadRequest("oauth state signature mismatch".into()));
    }

    let parts: Vec<&str> = payload.splitn(4, '|').collect();
    if parts.len() != 4 {
        return Err(Error::BadRequest("invalid oauth state".into()));
    }
    let exp: u64 = parts[3]
        .parse()
        .map_err(|_| Error::BadRequest("invalid oauth state".into()))?;
    let now = now_secs();
    if now > exp {
        return Err(Error::BadRequest("oauth state expired".into()));
    }
    Ok(FlowState {
        provider: parts[0].into(),
        code_verifier: parts[1].into(),
        nonce: parts[2].into(),
        exp,
    })
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn authorize_url(
    provider: &OauthProvider,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> Result<String> {
    let mut url = url::Url::parse(&provider.authorization_url)
        .map_err(|e| Error::Internal(format!("authorize url: {e}")))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", &provider.client_id);
        q.append_pair("redirect_uri", redirect_uri);
        q.append_pair("state", state);
        q.append_pair("code_challenge", code_challenge);
        q.append_pair("code_challenge_method", "S256");
        if !provider.scopes.is_empty() {
            q.append_pair("scope", &provider.scopes.join(" "));
        }
        for (k, v) in &provider.auth_params {
            q.append_pair(k, v);
        }
    }
    Ok(url.into())
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    #[serde(default)]
    access_token: String,
    refresh_token: Option<String>,
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    id_token: Option<String>,
    // GitHub sometimes returns error in body with 200.
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn exchange_code(
    http: &reqwest::Client,
    provider: &OauthProvider,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<OauthTokens> {
    let client_secret = provider.resolve_client_secret()?;
    let form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
        ("client_id", provider.client_id.clone()),
        ("client_secret", client_secret),
        ("code_verifier", code_verifier.to_string()),
    ];
    // GitHub wants Accept: application/json
    let res = http
        .post(&provider.token_url)
        .header("Accept", "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|e| Error::Internal(format!("oauth token: {e}")))?;

    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| Error::Internal(format!("oauth token body: {e}")))?;

    // Try JSON first; GitHub can also return form-urlencoded.
    let parsed: TokenResponse = if let Ok(t) = serde_json::from_str(&body) {
        t
    } else {
        parse_form_token(&body)?
    };

    if let Some(err) = parsed.error {
        return Err(Error::BadRequest(format!(
            "oauth token error: {err} {}",
            parsed.error_description.unwrap_or_default()
        )));
    }
    if !status.is_success() {
        return Err(Error::Internal(format!(
            "oauth token http {status}: {body}"
        )));
    }

    Ok(OauthTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        token_type: parsed.token_type,
        scope: parsed.scope,
        id_token: parsed.id_token,
    })
}

fn parse_form_token(body: &str) -> Result<TokenResponse> {
    let mut access_token = None;
    let mut refresh_token = None;
    let mut token_type = None;
    let mut scope = None;
    let mut id_token = None;
    let mut error = None;
    let mut error_description = None;
    for (k, v) in url::form_urlencoded::parse(body.as_bytes()) {
        match k.as_ref() {
            "access_token" => access_token = Some(v.into_owned()),
            "refresh_token" => refresh_token = Some(v.into_owned()),
            "token_type" => token_type = Some(v.into_owned()),
            "scope" => scope = Some(v.into_owned()),
            "id_token" => id_token = Some(v.into_owned()),
            "error" => error = Some(v.into_owned()),
            "error_description" => error_description = Some(v.into_owned()),
            _ => {}
        }
    }
    Ok(TokenResponse {
        access_token: access_token.unwrap_or_default(),
        refresh_token,
        token_type,
        scope,
        id_token,
        error,
        error_description,
    })
}

/// Resolve profile via userinfo, or from `id_token` when Apple / empty userinfo.
pub async fn resolve_profile(
    http: &reqwest::Client,
    provider: &OauthProvider,
    tokens: &OauthTokens,
) -> Result<(OauthProfile, serde_json::Value)> {
    if provider.profile_kind == ProfileKind::Apple || provider.userinfo_url.is_empty() {
        let id_token = tokens
            .id_token
            .as_deref()
            .ok_or_else(|| Error::Internal("oauth: missing id_token for profile".into()))?;
        let raw = decode_jwt_payload(id_token)?;
        let profile = parse_profile(provider.profile_kind, &raw)?;
        return Ok((profile, raw));
    }
    fetch_profile(http, provider, &tokens.access_token).await
}

pub async fn fetch_profile(
    http: &reqwest::Client,
    provider: &OauthProvider,
    access_token: &str,
) -> Result<(OauthProfile, serde_json::Value)> {
    let res = http
        .get(&provider.userinfo_url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Accept", "application/json")
        .header("User-Agent", "sova-passport-oauth")
        .send()
        .await
        .map_err(|e| Error::Internal(format!("oauth userinfo: {e}")))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(Error::Internal(format!(
            "oauth userinfo http {status}: {body}"
        )));
    }

    let raw: serde_json::Value = res
        .json()
        .await
        .map_err(|e| Error::Internal(format!("oauth userinfo json: {e}")))?;

    let mut profile = parse_profile(provider.profile_kind, &raw)?;

    // GitHub primary email may need a second call when email is null.
    if provider.profile_kind == ProfileKind::Github && profile.email.is_none() {
        if let Some(email) = fetch_github_email(http, access_token).await? {
            profile.email = Some(email);
        }
    }

    Ok((profile, raw))
}

async fn fetch_github_email(http: &reqwest::Client, access_token: &str) -> Result<Option<String>> {
    #[derive(Deserialize)]
    struct GhEmail {
        email: String,
        primary: bool,
        verified: bool,
    }
    let res = http
        .get("https://api.github.com/user/emails")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Accept", "application/json")
        .header("User-Agent", "sova-passport-oauth")
        .send()
        .await
        .map_err(|e| Error::Internal(format!("github emails: {e}")))?;
    if !res.status().is_success() {
        return Ok(None);
    }
    let list: Vec<GhEmail> = res.json().await.unwrap_or_default();
    let email = list
        .iter()
        .find(|e| e.primary && e.verified)
        .or_else(|| list.iter().find(|e| e.verified))
        .map(|e| e.email.clone());
    Ok(email)
}

/// Decode JWT payload without verifying the signature (MVP; state+PKCE bind the flow).
pub fn decode_jwt_payload(token: &str) -> Result<serde_json::Value> {
    let payload_b64 = token
        .split('.')
        .nth(1)
        .ok_or_else(|| Error::BadRequest("invalid id_token".into()))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| STANDARD.decode(payload_b64))
        .map_err(|_| Error::BadRequest("invalid id_token encoding".into()))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| Error::BadRequest(format!("invalid id_token json: {e}")))
}

pub fn parse_profile(kind: ProfileKind, raw: &serde_json::Value) -> Result<OauthProfile> {
    match kind {
        ProfileKind::Github => {
            let id = raw
                .get("id")
                .and_then(|v| {
                    v.as_i64()
                        .map(|n| n.to_string())
                        .or_else(|| v.as_str().map(str::to_string))
                })
                .ok_or_else(|| Error::Internal("github profile missing id".into()))?;
            Ok(OauthProfile {
                provider_user_id: id,
                email: raw.get("email").and_then(|v| v.as_str()).map(str::to_string),
                name: raw
                    .get("name")
                    .and_then(|v| v.as_str())
                    .or_else(|| raw.get("login").and_then(|v| v.as_str()))
                    .map(str::to_string),
                raw: raw.clone(),
            })
        }
        ProfileKind::Google | ProfileKind::Apple => {
            let id = raw
                .get("sub")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    Error::Internal(format!(
                        "{} profile missing sub",
                        match kind {
                            ProfileKind::Apple => "apple",
                            _ => "google",
                        }
                    ))
                })?
                .to_string();
            Ok(OauthProfile {
                provider_user_id: id,
                email: raw.get("email").and_then(|v| v.as_str()).map(str::to_string),
                name: raw.get("name").and_then(|v| v.as_str()).map(str::to_string),
                raw: raw.clone(),
            })
        }
        ProfileKind::Generic => {
            let id = raw
                .get("id")
                .or_else(|| raw.get("sub"))
                .and_then(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                })
                .ok_or_else(|| Error::Internal("oauth profile missing id/sub".into()))?;
            Ok(OauthProfile {
                provider_user_id: id,
                email: raw.get("email").and_then(|v| v.as_str()).map(str::to_string),
                name: raw.get("name").and_then(|v| v.as_str()).map(str::to_string),
                raw: raw.clone(),
            })
        }
    }
}

/// Encode cookie value (base64 of state token).
pub fn cookie_value(state_token: &str) -> String {
    STANDARD.encode(state_token.as_bytes())
}

pub fn cookie_decode(value: &str) -> Result<String> {
    let bytes = STANDARD
        .decode(value.trim())
        .map_err(|_| Error::BadRequest("invalid oauth cookie".into()))?;
    String::from_utf8(bytes).map_err(|_| Error::BadRequest("invalid oauth cookie".into()))
}

#[cfg(test)]
mod flow_tests {
    use super::*;
    use crate::oauth::OauthProvider;

    #[test]
    fn authorize_url_includes_auth_params() {
        let p = OauthProvider::google().client_id("cid");
        let url = authorize_url(&p, "http://localhost/cb", "st", "chal").unwrap();
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains("code_challenge=chal"));
    }

    #[test]
    fn parse_apple_profile_from_claims() {
        let raw = serde_json::json!({
            "sub": "001234.abcd",
            "email": "a@privaterelay.appleid.com",
        });
        let p = parse_profile(ProfileKind::Apple, &raw).unwrap();
        assert_eq!(p.provider_user_id, "001234.abcd");
        assert_eq!(p.email.as_deref(), Some("a@privaterelay.appleid.com"));
    }

    #[test]
    fn decode_jwt_payload_ok() {
        // header.payload.sig — payload = {"sub":"x"}
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"x"}"#);
        let token = format!("e30.{payload}.sig");
        let v = decode_jwt_payload(&token).unwrap();
        assert_eq!(v["sub"], "x");
    }
}

