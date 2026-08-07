//! Seed demo user + admin role after Postgres is up.

use ruvo::{
    assign_role, find_user_by_email, mark_email_verified, register_user, DbHandle, DbPool, Error,
};
use std::sync::Arc;

pub async fn seed_demo(state: Arc<ruvo::extend::StateMap>) -> Result<(), Error> {
    let pool = state
        .get::<DbPool>()
        .ok_or_else(|| Error::Internal("DbPool missing".into()))?;
    let conn = pool.get().await.map_err(Error::from)?;
    let db = DbHandle::Conn(conn);

    if find_user_by_email(&db, "demo@ruvo.local").await?.is_some() {
        return Ok(());
    }

    let u = register_user(&db, "demo@ruvo.local", "Demo User", "demo1234").await?;
    mark_email_verified(&db, u.id).await?;
    assign_role(&db, u.id, "admin").await?;
    tracing::info!("seeded demo@ruvo.local / demo1234 (admin)");
    Ok(())
}
