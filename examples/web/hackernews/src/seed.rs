//! Optional demo data: `cargo run -p hackernews -- seed`
//!
//! Seed CLI expects [`sova::Error`] (core), not facade [`sova::AppError`].

use crate::db;
use sea_orm::{EntityTrait, PaginatorTrait};
use sova::extend::StateMap;
use sova::{find_user_by_email, register_user, DbError, DbHandle, DbPool, Error};
use std::sync::Arc;

fn core(err: sova::AppError) -> Error {
    match err {
        sova::AppError::Core(c) => c,
    }
}

pub async fn run(state: Arc<StateMap>) -> Result<(), Error> {
    let pool = state
        .get::<DbPool>()
        .ok_or_else(|| Error::Internal("DbPool missing for seed".into()))?;
    let conn = pool.get().await.map_err(Error::from)?;
    let db = DbHandle::Conn(conn);

    let n = crate::entity::story::Entity::find()
        .count(&db)
        .await
        .map_err(DbError::from)?;
    if n > 0 {
        return Ok(());
    }

    let user_id = ensure_demo_user(&db).await?;

    db::create_story(
        &db,
        user_id,
        "Welcome to Sova News".into(),
        Some("https://s00d.github.io/sova/".into()),
        None,
    )
    .await
    .map_err(core)?;
    db::create_story(
        &db,
        user_id,
        "Ask SN: what should we build next?".into(),
        None,
        Some("Drop ideas in the comments.".into()),
    )
    .await
    .map_err(core)?;
    Ok(())
}

async fn ensure_demo_user(db: &DbHandle) -> Result<i64, Error> {
    if let Some(u) = find_user_by_email(db, "demo@sova.news").await? {
        return Ok(u.id);
    }
    let u = register_user(db, "demo@sova.news", "demo", "demo1234").await?;
    Ok(u.id)
}
