//! Persist OAuth accounts linked to `auth_users`.

use crate::entity::{oauth_account, user};
use crate::store::{create_oauth_user, find_user_by_email, find_user_by_id, AuthUser};
use chrono::Utc;
use sova_core::{Error, Result};
use sova_db::{DbError, DbHandle};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use super::{OauthProfile, OauthTokens};

pub async fn find_account(
    db: &DbHandle,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<oauth_account::Model>> {
    oauth_account::Entity::find()
        .filter(oauth_account::Column::Provider.eq(provider))
        .filter(oauth_account::Column::ProviderUserId.eq(provider_user_id))
        .one(db)
        .await
        .map_err(db_err)
}

pub async fn find_or_create_user(
    db: &DbHandle,
    provider: &str,
    profile: &OauthProfile,
    tokens: &OauthTokens,
) -> Result<AuthUser> {
    if let Some(acc) = find_account(db, provider, &profile.provider_user_id).await? {
        // Update tokens / profile.
        let mut am: oauth_account::ActiveModel = acc.clone().into();
        am.access_token = Set(Some(tokens.access_token.clone()));
        am.refresh_token = Set(tokens.refresh_token.clone());
        am.profile_json = Set(Some(profile.raw.to_string()));
        am.updated_at = Set(Utc::now());
        am.update(db).await.map_err(db_err)?;
        let user = find_user_by_id(db, acc.user_id)
            .await?
            .ok_or(Error::Unauthorized)?;
        return Ok(AuthUser::from(&user));
    }

    let email = profile
        .email
        .clone()
        .unwrap_or_else(|| format!("{}:{}@oauth.local", provider, profile.provider_user_id));

    let user = if let Some(existing) = find_user_by_email(db, &email).await? {
        existing
    } else {
        create_oauth_user(db, &email).await?
    };

    let now = Utc::now();
    let row = oauth_account::ActiveModel {
        user_id: Set(user.id),
        provider: Set(provider.to_string()),
        provider_user_id: Set(profile.provider_user_id.clone()),
        access_token: Set(Some(tokens.access_token.clone())),
        refresh_token: Set(tokens.refresh_token.clone()),
        profile_json: Set(Some(profile.raw.to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    row.insert(db).await.map_err(db_err)?;
    Ok(AuthUser::from(&user))
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::from(DbError::from(e))
}

// keep user import used
#[allow(dead_code)]
fn _user_ty(_: &user::Model) {}
