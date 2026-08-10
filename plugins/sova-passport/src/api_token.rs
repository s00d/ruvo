//! Personal access tokens (PAT) for machine clients.

use crate::entity::{api_token, user};
use crate::store::{find_user_by_id, hash_token, AuthUser};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use sova_core::{Error, EventBus, Result};
use sova_db::{DbError, DbHandle};

pub const PAT_PREFIX: &str = "svpat_";

/// Options for [`create_api_token`].
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApiToken {
    pub name: String,
    #[serde(default)]
    pub abilities: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Returned once at creation — includes plaintext `token`.
#[derive(Debug, Clone, Serialize)]
pub struct CreatedApiToken {
    pub id: i64,
    pub name: String,
    pub token: String,
    pub prefix: String,
    pub abilities: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Safe list/detail view (no plaintext).
#[derive(Debug, Clone, Serialize)]
pub struct ApiTokenRow {
    pub id: i64,
    pub name: String,
    pub prefix: String,
    pub abilities: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Attached to the request when auth was via PAT (not JWT).
#[derive(Debug, Clone)]
pub struct ApiTokenInfo {
    pub id: i64,
    pub abilities: Vec<String>,
}

/// `[]` abilities = full access; otherwise exact string match.
pub fn token_can(abilities: &[String], ability: &str) -> bool {
    abilities.is_empty() || abilities.iter().any(|a| a == ability)
}

pub fn parse_abilities(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn encode_abilities(abilities: &[String]) -> String {
    serde_json::to_string(abilities).unwrap_or_else(|_| "[]".into())
}

fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    let _ = getrandom::getrandom(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Build plaintext `svpat_<prefix>_<secret>` and return `(full, prefix)`.
pub fn mint_pat_plaintext() -> (String, String) {
    let prefix = random_hex(4); // 8 hex chars
    let secret = random_hex(32);
    let full = format!("{PAT_PREFIX}{prefix}_{secret}");
    (full, prefix)
}

/// Parse `svpat_<prefix>_<secret>` → prefix.
pub fn pat_prefix(raw: &str) -> Option<&str> {
    let rest = raw.strip_prefix(PAT_PREFIX)?;
    rest.split_once('_').map(|(p, _)| p)
}

pub fn is_pat(raw: &str) -> bool {
    raw.starts_with(PAT_PREFIX)
}

pub async fn create_api_token(
    db: &DbHandle,
    user_id: i64,
    opts: CreateApiToken,
) -> Result<CreatedApiToken> {
    let name = opts.name.trim();
    if name.is_empty() {
        return Err(Error::BadRequest("token name required".into()));
    }
    let (plaintext, prefix) = mint_pat_plaintext();
    let now = Utc::now();
    let abilities = opts.abilities;
    let model = api_token::ActiveModel {
        user_id: Set(user_id),
        name: Set(name.to_string()),
        token_prefix: Set(prefix.clone()),
        token_hash: Set(hash_token(&plaintext)),
        abilities: Set(encode_abilities(&abilities)),
        expires_at: Set(opts.expires_at),
        last_used_at: Set(None),
        revoked_at: Set(None),
        created_at: Set(now),
        ..Default::default()
    };
    let row = model.insert(db).await.map_err(db_err)?;
    Ok(CreatedApiToken {
        id: row.id,
        name: row.name,
        token: plaintext,
        prefix,
        abilities,
        expires_at: row.expires_at,
        created_at: row.created_at,
    })
}

pub async fn list_api_tokens(db: &DbHandle, user_id: i64) -> Result<Vec<ApiTokenRow>> {
    let rows = api_token::Entity::find()
        .filter(api_token::Column::UserId.eq(user_id))
        .filter(api_token::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_err(db_err)?;
    Ok(rows
        .into_iter()
        .map(|r| ApiTokenRow {
            id: r.id,
            name: r.name,
            prefix: r.token_prefix,
            abilities: parse_abilities(&r.abilities),
            expires_at: r.expires_at,
            last_used_at: r.last_used_at,
            created_at: r.created_at,
        })
        .collect())
}

pub async fn revoke_api_token(
    db: &DbHandle,
    user_id: i64,
    id: i64,
    events: Option<&EventBus>,
) -> Result<bool> {
    let Some(row) = api_token::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_err)?
    else {
        return Ok(false);
    };
    if row.user_id != user_id || row.revoked_at.is_some() {
        return Ok(false);
    }
    let mut am: api_token::ActiveModel = row.into();
    am.revoked_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(db_err)?;
    if let Some(bus) = events {
        bus.dispatch(crate::ApiTokenRevoked {
            user_id,
            token_id: id,
        });
    }
    Ok(true)
}

/// Validate PAT; returns user + token info if valid.
pub async fn user_for_api_token(db: &DbHandle, raw: &str) -> Result<(user::Model, ApiTokenInfo)> {
    let prefix = pat_prefix(raw).ok_or(Error::Unauthorized)?;
    let hash = hash_token(raw);
    let candidates = api_token::Entity::find()
        .filter(api_token::Column::TokenPrefix.eq(prefix))
        .filter(api_token::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_err(db_err)?;

    let row = candidates
        .into_iter()
        .find(|r| constant_eq(r.token_hash.as_bytes(), hash.as_bytes()))
        .ok_or(Error::Unauthorized)?;

    if let Some(exp) = row.expires_at {
        if exp < Utc::now() {
            return Err(Error::Unauthorized);
        }
    }

    let abilities = parse_abilities(&row.abilities);
    let info = ApiTokenInfo {
        id: row.id,
        abilities,
    };

    // Touch last_used (best-effort).
    let mut am: api_token::ActiveModel = row.clone().into();
    am.last_used_at = Set(Some(Utc::now()));
    let _ = am.update(db).await;

    let user = find_user_by_id(db, row.user_id)
        .await?
        .ok_or(Error::Unauthorized)?;
    Ok((user, info))
}

fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::from(DbError::from(e))
}

/// Convenience: AuthUser from PAT.
pub async fn auth_user_for_api_token(db: &DbHandle, raw: &str) -> Result<(AuthUser, ApiTokenInfo)> {
    let (user, info) = user_for_api_token(db, raw).await?;
    Ok((AuthUser::from(&user), info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_can_empty_is_full_access() {
        assert!(token_can(&[], "anything"));
        assert!(!token_can(&["a".into()], "b"));
        assert!(token_can(&["a".into()], "a"));
    }

    #[test]
    fn mint_and_parse_prefix() {
        let (full, prefix) = mint_pat_plaintext();
        assert!(full.starts_with(PAT_PREFIX));
        assert_eq!(pat_prefix(&full), Some(prefix.as_str()));
        assert!(is_pat(&full));
        assert!(!is_pat("eyJhbGciOiJIUzI1NiJ9.x.y"));
    }
}
