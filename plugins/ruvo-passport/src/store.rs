//! Persistence helpers for users + refresh tokens.

use crate::entity::{refresh_token, user};
use crate::password::{hash_password, verify_password};
use chrono::{Duration, Utc};
use ruvo_core::{Error, Request, Result};
use ruvo_db::{DbError, DbExt, DbHandle};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

const MIN_PASSWORD_LEN: usize = 8;

/// Public user payload (no password hash).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub user: AuthUser,
}

pub fn hash_refresh_token(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn random_refresh_token() -> String {
    let mut buf = [0u8; 32];
    let _ = getrandom::getrandom(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn validate_password(password: &str) -> Result<()> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(Error::BadRequest(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<()> {
    let email = email.trim();
    if email.is_empty() || !email.contains('@') {
        return Err(Error::BadRequest("invalid email".into()));
    }
    Ok(())
}

impl From<&user::Model> for AuthUser {
    fn from(m: &user::Model) -> Self {
        Self {
            id: m.id,
            email: m.email.clone(),
        }
    }
}

pub async fn find_user_by_id(db: &DbHandle, id: i64) -> Result<Option<user::Model>> {
    user::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_err)
}

pub async fn find_user_by_email(db: &DbHandle, email: &str) -> Result<Option<user::Model>> {
    user::Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(db_err)
}

pub async fn register_user(db: &DbHandle, email: &str, password: &str) -> Result<user::Model> {
    validate_email(email)?;
    validate_password(password)?;
    let email = email.trim().to_lowercase();

    if find_user_by_email(db, &email).await?.is_some() {
        return Err(Error::custom(409, "email already registered"));
    }

    let model = user::ActiveModel {
        email: Set(email),
        password_hash: Set(Some(hash_password(password)?)),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    model.insert(db).await.map_err(db_err)
}

pub async fn authenticate(db: &DbHandle, email: &str, password: &str) -> Result<user::Model> {
    let email = email.trim().to_lowercase();
    let user = find_user_by_email(db, &email)
        .await?
        .ok_or(Error::Unauthorized)?;
    let Some(hash) = user.password_hash.as_deref() else {
        return Err(Error::Unauthorized);
    };
    if !verify_password(password, hash)? {
        return Err(Error::Unauthorized);
    }
    Ok(user)
}

pub async fn store_refresh(
    db: &DbHandle,
    user_id: i64,
    raw_token: &str,
    ttl_secs: i64,
) -> Result<()> {
    let now = Utc::now();
    let model = refresh_token::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(hash_refresh_token(raw_token)),
        expires_at: Set(now + Duration::seconds(ttl_secs)),
        revoked_at: Set(None),
        created_at: Set(now),
        ..Default::default()
    };
    model.insert(db).await.map_err(db_err)?;
    Ok(())
}

pub async fn revoke_refresh(db: &DbHandle, raw_token: &str) -> Result<bool> {
    let hash = hash_refresh_token(raw_token);
    let Some(row) = refresh_token::Entity::find()
        .filter(refresh_token::Column::TokenHash.eq(hash))
        .one(db)
        .await
        .map_err(db_err)?
    else {
        return Ok(false);
    };
    if row.revoked_at.is_some() {
        return Ok(false);
    }
    let mut am: refresh_token::ActiveModel = row.into();
    am.revoked_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(db_err)?;
    Ok(true)
}

/// Validate refresh token; returns user if valid and not revoked/expired.
pub async fn user_for_refresh(db: &DbHandle, raw_token: &str) -> Result<user::Model> {
    let hash = hash_refresh_token(raw_token);
    let row = refresh_token::Entity::find()
        .filter(refresh_token::Column::TokenHash.eq(hash))
        .one(db)
        .await
        .map_err(db_err)?
        .ok_or(Error::Unauthorized)?;

    if row.revoked_at.is_some() {
        return Err(Error::Unauthorized);
    }
    if row.expires_at < Utc::now() {
        return Err(Error::Unauthorized);
    }

    find_user_by_id(db, row.user_id)
        .await?
        .ok_or(Error::Unauthorized)
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::from(DbError::from(e))
}

/// Issue access + refresh JWT pair (requires [`crate::JwtAuth`] installed).
pub async fn issue_token_pair(
    req: &Request,
    user: &user::Model,
) -> Result<TokenPair> {
    use crate::jwt_auth::JwtAuthState;
    let state = req.try_state::<JwtAuthState>().ok_or_else(|| {
        Error::Internal("JwtAuth is not installed (needed to issue tokens)".into())
    })?;
    let state = (*state).clone();
    let access = state
        .jwt
        .issue_access(user.id.to_string(), state.access_ttl)
        .map_err(|e| Error::Internal(e.to_string()))?;
    let refresh = random_refresh_token();
    store_refresh(req.db(), user.id, &refresh, state.refresh_ttl as i64).await?;
    Ok(TokenPair {
        access_token: access,
        refresh_token: refresh,
        token_type: "Bearer",
        expires_in: state.access_ttl,
        user: AuthUser::from(user),
    })
}

/// Create password-less user (OAuth).
#[cfg(feature = "oauth")]
pub async fn create_oauth_user(db: &DbHandle, email: &str) -> Result<user::Model> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(Error::BadRequest("invalid email".into()));
    }
    if find_user_by_email(db, &email).await?.is_some() {
        return Err(Error::custom(409, "email already registered"));
    }
    let model = user::ActiveModel {
        email: Set(email),
        password_hash: Set(None),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    model.insert(db).await.map_err(db_err)
}
