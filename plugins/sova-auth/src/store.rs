//! Domain helpers: users, roles, tokens.

use crate::entity::{permission, permission_role, reset_token, role, role_user, user};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use sha2::{Digest, Sha256};
use sova_core::{Error, Result};
use sova_db::{DbError, DbHandle};
use sova_passport::{hash_password, verify_password};

/// Session principal for Fortify (web + API).
#[derive(Clone, Debug, serde::Serialize)]
pub struct CurrentUser {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub avatar_path: Option<String>,
    pub email_verified: bool,
    pub two_factor_enabled: bool,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

impl CurrentUser {
    pub fn has_role(&self, slug: &str) -> bool {
        self.roles.iter().any(|r| r == slug)
    }

    pub fn has_permission(&self, slug: &str) -> bool {
        self.has_role("admin") || self.permissions.iter().any(|p| p == slug)
    }
}

pub fn hash_token(raw: &str) -> String {
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn random_token() -> String {
    let mut buf = [0u8; 32];
    let _ = getrandom::getrandom(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::from(DbError::from(e))
}

pub async fn find_user_by_id(db: &DbHandle, id: i64) -> Result<Option<user::Model>> {
    user::Entity::find_by_id(id).one(db).await.map_err(db_err)
}

pub async fn find_user_by_email(db: &DbHandle, email: &str) -> Result<Option<user::Model>> {
    user::Entity::find()
        .filter(user::Column::Email.eq(email.trim().to_lowercase()))
        .one(db)
        .await
        .map_err(db_err)
}

pub async fn load_current_user(db: &DbHandle, id: i64) -> Result<Option<CurrentUser>> {
    let Some(u) = find_user_by_id(db, id).await? else {
        return Ok(None);
    };
    let (roles, permissions) = load_rbac(db, u.id).await?;
    Ok(Some(CurrentUser {
        id: u.id,
        email: u.email,
        name: u.name,
        avatar_path: u.avatar_path,
        email_verified: u.email_verified_at.is_some(),
        two_factor_enabled: u.two_factor_confirmed_at.is_some(),
        roles,
        permissions,
    }))
}

pub async fn load_rbac(db: &DbHandle, user_id: i64) -> Result<(Vec<String>, Vec<String>)> {
    let links = role_user::Entity::find()
        .filter(role_user::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(db_err)?;
    let mut roles = Vec::new();
    let mut perm_slugs = Vec::new();
    for link in links {
        if let Some(r) = role::Entity::find_by_id(link.role_id)
            .one(db)
            .await
            .map_err(db_err)?
        {
            roles.push(r.slug.clone());
            let prs = permission_role::Entity::find()
                .filter(permission_role::Column::RoleId.eq(r.id))
                .all(db)
                .await
                .map_err(db_err)?;
            for pr in prs {
                if let Some(p) = permission::Entity::find_by_id(pr.permission_id)
                    .one(db)
                    .await
                    .map_err(db_err)?
                {
                    if !perm_slugs.contains(&p.slug) {
                        perm_slugs.push(p.slug);
                    }
                }
            }
        }
    }
    Ok((roles, perm_slugs))
}

pub async fn assign_role(db: &DbHandle, user_id: i64, role_slug: &str) -> Result<()> {
    let Some(r) = role::Entity::find()
        .filter(role::Column::Slug.eq(role_slug))
        .one(db)
        .await
        .map_err(db_err)?
    else {
        return Err(Error::Internal(format!("role `{role_slug}` missing")));
    };
    let exists = role_user::Entity::find()
        .filter(role_user::Column::UserId.eq(user_id))
        .filter(role_user::Column::RoleId.eq(r.id))
        .one(db)
        .await
        .map_err(db_err)?;
    if exists.is_some() {
        return Ok(());
    }
    role_user::ActiveModel {
        user_id: Set(user_id),
        role_id: Set(r.id),
    }
    .insert(db)
    .await
    .map_err(db_err)?;
    Ok(())
}

pub async fn register_user(
    db: &DbHandle,
    email: &str,
    name: &str,
    password: &str,
) -> Result<user::Model> {
    let email = email.trim().to_lowercase();
    if find_user_by_email(db, &email).await?.is_some() {
        return Err(Error::custom(409, "email already registered"));
    }
    if password.len() < 8 {
        return Err(Error::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    let model = user::ActiveModel {
        email: Set(email),
        name: Set(name.trim().to_string()),
        password_hash: Set(Some(hash_password(password)?)),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    let u = model.insert(db).await.map_err(db_err)?;
    assign_role(db, u.id, "user").await?;
    Ok(u)
}

pub async fn attempt_login(db: &DbHandle, email: &str, password: &str) -> Result<user::Model> {
    let u = find_user_by_email(db, email)
        .await?
        .ok_or(Error::Unauthorized)?;
    let Some(hash) = u.password_hash.as_deref() else {
        return Err(Error::Unauthorized);
    };
    if !verify_password(password, hash)? {
        return Err(Error::Unauthorized);
    }
    Ok(u)
}

pub async fn store_reset_token(db: &DbHandle, email: &str, raw: &str) -> Result<()> {
    let email = email.trim().to_lowercase();
    // upsert
    if let Some(existing) = reset_token::Entity::find_by_id(email.clone())
        .one(db)
        .await
        .map_err(db_err)?
    {
        let mut am: reset_token::ActiveModel = existing.into();
        am.token_hash = Set(hash_token(raw));
        am.created_at = Set(Utc::now());
        am.update(db).await.map_err(db_err)?;
    } else {
        reset_token::ActiveModel {
            email: Set(email),
            token_hash: Set(hash_token(raw)),
            created_at: Set(Utc::now()),
        }
        .insert(db)
        .await
        .map_err(db_err)?;
    }
    Ok(())
}

pub async fn consume_reset_token(db: &DbHandle, email: &str, raw: &str) -> Result<()> {
    let email = email.trim().to_lowercase();
    let row = reset_token::Entity::find_by_id(email.clone())
        .one(db)
        .await
        .map_err(db_err)?
        .ok_or(Error::BadRequest("invalid or expired reset token".into()))?;
    if row.token_hash != hash_token(raw) {
        return Err(Error::BadRequest("invalid or expired reset token".into()));
    }
    if row.created_at + Duration::hours(1) < Utc::now() {
        return Err(Error::BadRequest("invalid or expired reset token".into()));
    }
    reset_token::Entity::delete_by_id(email)
        .exec(db)
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn set_password(db: &DbHandle, user_id: i64, password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(Error::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    let Some(u) = find_user_by_id(db, user_id).await? else {
        return Err(Error::NotFound);
    };
    let mut am: user::ActiveModel = u.into();
    am.password_hash = Set(Some(hash_password(password)?));
    am.update(db).await.map_err(db_err)?;
    Ok(())
}

pub async fn mark_email_verified(db: &DbHandle, user_id: i64) -> Result<()> {
    let Some(u) = find_user_by_id(db, user_id).await? else {
        return Err(Error::NotFound);
    };
    let mut am: user::ActiveModel = u.into();
    am.email_verified_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(db_err)?;
    Ok(())
}

pub async fn update_profile(
    db: &DbHandle,
    user_id: i64,
    name: &str,
    email: &str,
) -> Result<user::Model> {
    let Some(u) = find_user_by_id(db, user_id).await? else {
        return Err(Error::NotFound);
    };
    let email = email.trim().to_lowercase();
    if email != u.email && find_user_by_email(db, &email).await?.is_some() {
        return Err(Error::custom(409, "email already registered"));
    }
    let email_changed = email != u.email;
    let mut am: user::ActiveModel = u.into();
    am.name = Set(name.trim().to_string());
    am.email = Set(email);
    if email_changed {
        am.email_verified_at = Set(None);
    }
    am.update(db).await.map_err(db_err)
}

pub async fn set_avatar(db: &DbHandle, user_id: i64, path: Option<String>) -> Result<()> {
    let Some(u) = find_user_by_id(db, user_id).await? else {
        return Err(Error::NotFound);
    };
    let mut am: user::ActiveModel = u.into();
    am.avatar_path = Set(path);
    am.update(db).await.map_err(db_err)?;
    Ok(())
}

pub async fn enable_2fa_secret(
    db: &DbHandle,
    user_id: i64,
    secret: &str,
    codes_json: &str,
) -> Result<()> {
    let Some(u) = find_user_by_id(db, user_id).await? else {
        return Err(Error::NotFound);
    };
    let mut am: user::ActiveModel = u.into();
    am.two_factor_secret = Set(Some(secret.to_string()));
    am.two_factor_recovery_codes = Set(Some(codes_json.to_string()));
    am.two_factor_confirmed_at = Set(None);
    am.update(db).await.map_err(db_err)?;
    Ok(())
}

pub async fn set_recovery_codes(db: &DbHandle, user_id: i64, codes_json: &str) -> Result<()> {
    let Some(u) = find_user_by_id(db, user_id).await? else {
        return Err(Error::NotFound);
    };
    let mut am: user::ActiveModel = u.into();
    am.two_factor_recovery_codes = Set(Some(codes_json.to_string()));
    am.update(db).await.map_err(db_err)?;
    Ok(())
}

pub async fn confirm_2fa(db: &DbHandle, user_id: i64) -> Result<()> {
    let Some(u) = find_user_by_id(db, user_id).await? else {
        return Err(Error::NotFound);
    };
    let mut am: user::ActiveModel = u.into();
    am.two_factor_confirmed_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(db_err)?;
    Ok(())
}

pub async fn disable_2fa(db: &DbHandle, user_id: i64) -> Result<()> {
    let Some(u) = find_user_by_id(db, user_id).await? else {
        return Err(Error::NotFound);
    };
    let mut am: user::ActiveModel = u.into();
    am.two_factor_secret = Set(None);
    am.two_factor_recovery_codes = Set(None);
    am.two_factor_confirmed_at = Set(None);
    am.update(db).await.map_err(db_err)?;
    Ok(())
}

pub async fn list_roles(db: &DbHandle) -> Result<Vec<role::Model>> {
    role::Entity::find().all(db).await.map_err(db_err)
}

pub async fn list_permissions(db: &DbHandle) -> Result<Vec<permission::Model>> {
    permission::Entity::find().all(db).await.map_err(db_err)
}

pub fn is_system_role_slug(slug: &str) -> bool {
    matches!(slug, "admin" | "user")
}

pub async fn find_role(db: &DbHandle, id: i64) -> Result<Option<role::Model>> {
    role::Entity::find_by_id(id).one(db).await.map_err(db_err)
}

pub async fn find_permission(db: &DbHandle, id: i64) -> Result<Option<permission::Model>> {
    permission::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(db_err)
}

pub async fn role_permission_ids(db: &DbHandle, role_id: i64) -> Result<Vec<i64>> {
    let rows = permission_role::Entity::find()
        .filter(permission_role::Column::RoleId.eq(role_id))
        .all(db)
        .await
        .map_err(db_err)?;
    Ok(rows.into_iter().map(|r| r.permission_id).collect())
}

pub async fn user_role_ids(db: &DbHandle, user_id: i64) -> Result<Vec<i64>> {
    let rows = role_user::Entity::find()
        .filter(role_user::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(db_err)?;
    Ok(rows.into_iter().map(|r| r.role_id).collect())
}

/// All user ids that have the given role slug (including via direct role_user).
pub async fn user_ids_with_role(db: &DbHandle, role_slug: &str) -> Result<Vec<i64>> {
    let Some(r) = role::Entity::find()
        .filter(role::Column::Slug.eq(role_slug))
        .one(db)
        .await
        .map_err(db_err)?
    else {
        return Ok(vec![]);
    };
    let rows = role_user::Entity::find()
        .filter(role_user::Column::RoleId.eq(r.id))
        .all(db)
        .await
        .map_err(db_err)?;
    Ok(rows.into_iter().map(|r| r.user_id).collect())
}

/// Users who hold `permission_slug` via any role, plus all `admin` role members.
pub async fn user_ids_with_permission(db: &DbHandle, permission_slug: &str) -> Result<Vec<i64>> {
    let mut ids = user_ids_with_role(db, "admin").await?;
    if let Some(p) = permission::Entity::find()
        .filter(permission::Column::Slug.eq(permission_slug))
        .one(db)
        .await
        .map_err(db_err)?
    {
        let links = permission_role::Entity::find()
            .filter(permission_role::Column::PermissionId.eq(p.id))
            .all(db)
            .await
            .map_err(db_err)?;
        for link in links {
            let rows = role_user::Entity::find()
                .filter(role_user::Column::RoleId.eq(link.role_id))
                .all(db)
                .await
                .map_err(db_err)?;
            for row in rows {
                if !ids.contains(&row.user_id) {
                    ids.push(row.user_id);
                }
            }
        }
    }
    Ok(ids)
}

pub async fn create_role(db: &DbHandle, name: &str, slug: &str) -> Result<role::Model> {
    let slug = slug.trim().to_lowercase();
    if slug.is_empty() || name.trim().is_empty() {
        return Err(Error::BadRequest("name and slug required".into()));
    }
    role::ActiveModel {
        id: Default::default(),
        name: Set(name.trim().into()),
        slug: Set(slug),
    }
    .insert(db)
    .await
    .map_err(db_err)
}

pub async fn update_role(
    db: &DbHandle,
    id: i64,
    name: Option<&str>,
    slug: Option<&str>,
    allow_system: bool,
) -> Result<role::Model> {
    let Some(r) = find_role(db, id).await? else {
        return Err(Error::NotFound);
    };
    if is_system_role_slug(&r.slug)
        && !allow_system
        && slug.is_some_and(|s| s.trim().to_lowercase() != r.slug)
    {
        return Err(Error::BadRequest("cannot rename system role slug".into()));
    }
    let mut am: role::ActiveModel = r.into();
    if let Some(n) = name {
        am.name = Set(n.trim().into());
    }
    if let Some(s) = slug {
        am.slug = Set(s.trim().to_lowercase());
    }
    am.update(db).await.map_err(db_err)
}

pub async fn delete_role(db: &DbHandle, id: i64, allow_system: bool) -> Result<()> {
    let Some(r) = find_role(db, id).await? else {
        return Err(Error::NotFound);
    };
    if is_system_role_slug(&r.slug) && !allow_system {
        return Err(Error::BadRequest("cannot delete system role".into()));
    }
    permission_role::Entity::delete_many()
        .filter(permission_role::Column::RoleId.eq(id))
        .exec(db)
        .await
        .map_err(db_err)?;
    role_user::Entity::delete_many()
        .filter(role_user::Column::RoleId.eq(id))
        .exec(db)
        .await
        .map_err(db_err)?;
    role::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn sync_role_permissions(
    db: &DbHandle,
    role_id: i64,
    permission_ids: &[i64],
) -> Result<()> {
    if find_role(db, role_id).await?.is_none() {
        return Err(Error::NotFound);
    }
    permission_role::Entity::delete_many()
        .filter(permission_role::Column::RoleId.eq(role_id))
        .exec(db)
        .await
        .map_err(db_err)?;
    for pid in permission_ids {
        if find_permission(db, *pid).await?.is_none() {
            return Err(Error::BadRequest(format!("permission {pid} not found")));
        }
        permission_role::ActiveModel {
            role_id: Set(role_id),
            permission_id: Set(*pid),
        }
        .insert(db)
        .await
        .map_err(db_err)?;
    }
    Ok(())
}

pub async fn create_permission(db: &DbHandle, name: &str, slug: &str) -> Result<permission::Model> {
    let slug = slug.trim().to_lowercase();
    if slug.is_empty() || name.trim().is_empty() {
        return Err(Error::BadRequest("name and slug required".into()));
    }
    permission::ActiveModel {
        id: Default::default(),
        name: Set(name.trim().into()),
        slug: Set(slug),
    }
    .insert(db)
    .await
    .map_err(db_err)
}

pub async fn update_permission(
    db: &DbHandle,
    id: i64,
    name: Option<&str>,
    slug: Option<&str>,
) -> Result<permission::Model> {
    let Some(p) = find_permission(db, id).await? else {
        return Err(Error::NotFound);
    };
    let mut am: permission::ActiveModel = p.into();
    if let Some(n) = name {
        am.name = Set(n.trim().into());
    }
    if let Some(s) = slug {
        am.slug = Set(s.trim().to_lowercase());
    }
    am.update(db).await.map_err(db_err)
}

pub async fn delete_permission(db: &DbHandle, id: i64) -> Result<()> {
    if find_permission(db, id).await?.is_none() {
        return Err(Error::NotFound);
    }
    permission_role::Entity::delete_many()
        .filter(permission_role::Column::PermissionId.eq(id))
        .exec(db)
        .await
        .map_err(db_err)?;
    permission::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn set_user_roles(db: &DbHandle, user_id: i64, role_ids: &[i64]) -> Result<()> {
    if find_user_by_id(db, user_id).await?.is_none() {
        return Err(Error::NotFound);
    }
    role_user::Entity::delete_many()
        .filter(role_user::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(db_err)?;
    for rid in role_ids {
        if find_role(db, *rid).await?.is_none() {
            return Err(Error::BadRequest(format!("role {rid} not found")));
        }
        role_user::ActiveModel {
            user_id: Set(user_id),
            role_id: Set(*rid),
        }
        .insert(db)
        .await
        .map_err(db_err)?;
    }
    Ok(())
}

pub async fn revoke_role(db: &DbHandle, user_id: i64, role_id: i64) -> Result<()> {
    role_user::Entity::delete_many()
        .filter(role_user::Column::UserId.eq(user_id))
        .filter(role_user::Column::RoleId.eq(role_id))
        .exec(db)
        .await
        .map_err(db_err)?;
    Ok(())
}
