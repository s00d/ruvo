//! Audience resolution (feature `auth`).

use sova_core::Result;
use sova_db::DbHandle;

pub async fn user_ids_with_role(db: &DbHandle, slug: &str) -> Result<Vec<i64>> {
    sova_auth::user_ids_with_role(db, slug).await
}

pub async fn user_ids_with_permission(db: &DbHandle, slug: &str) -> Result<Vec<i64>> {
    sova_auth::user_ids_with_permission(db, slug).await
}
