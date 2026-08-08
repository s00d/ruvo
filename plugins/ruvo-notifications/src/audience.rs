//! Audience resolution (feature `auth`).

use ruvo_core::Result;
use ruvo_db::DbHandle;

pub async fn user_ids_with_role(db: &DbHandle, slug: &str) -> Result<Vec<i64>> {
    ruvo_auth::user_ids_with_role(db, slug).await
}

pub async fn user_ids_with_permission(db: &DbHandle, slug: &str) -> Result<Vec<i64>> {
    ruvo_auth::user_ids_with_permission(db, slug).await
}
