//! Demo data on first boot (`seed_on_startup` / `cargo run -p hackernews -- seed`).

use crate::db;
use sea_orm::{EntityTrait, PaginatorTrait};
use sova::extend::StateMap;
use sova::{find_user_by_email, register_user, DbError, DbHandle, DbPool, Result};
use std::sync::Arc;

pub async fn run(state: Arc<StateMap>) -> Result<()> {
    let pool = state
        .get::<DbPool>()
        .ok_or_else(|| sova::Error::Internal("DbPool missing for seed".into()))?;
    let conn = pool.get()?;
    let db = DbHandle::Conn(conn);

    let n = crate::entity::story::Entity::find()
        .count(&db)
        .await
        .map_err(DbError::from)?;
    if n > 0 {
        return Ok(());
    }

    let demo = ensure_user(&db, "demo@sova.news", "demo", "demo1234").await?;
    let alice = ensure_user(&db, "alice@sova.news", "alice", "alice1234").await?;
    let bob = ensure_user(&db, "bob@sova.news", "bob", "bob12345").await?;

    let welcome = db::create_story(
        &db,
        demo,
        "Welcome to Sova News".into(),
        Some("https://s00d.github.io/sova/".into()),
        None,
    )
    .await?;
    let ask = db::create_story(
        &db,
        alice,
        "Ask SN: what should we build next?".into(),
        None,
        Some("Drop ideas in the comments — plugins, DX, docs…".into()),
    )
    .await?;
    let show = db::create_story(
        &db,
        bob,
        "Show SN: cargo-sovax scaffolding".into(),
        Some("https://s00d.github.io/sova/guide/cargo-sovax".into()),
        None,
    )
    .await?;
    let _perf = db::create_story(
        &db,
        demo,
        "Notes on MemoryStore sharding".into(),
        None,
        Some("Default shards are fine for HTTP; avoid with_shards(1) under load.".into()),
    )
    .await?;

    let _ = db::upvote(&db, alice, welcome.id).await?;
    let _ = db::upvote(&db, bob, welcome.id).await?;
    let _ = db::upvote(&db, demo, ask.id).await?;
    let _ = db::upvote(&db, alice, show.id).await?;

    db::add_comment(
        &db,
        alice,
        welcome.id,
        "Nice — sqlite + migrate_on_startup just works.".into(),
    )
    .await?;
    db::add_comment(
        &db,
        bob,
        welcome.id,
        "Try registering another account and submitting a link.".into(),
    )
    .await?;
    db::add_comment(
        &db,
        demo,
        ask.id,
        "I'd love more realtime examples (ws / sse).".into(),
    )
    .await?;

    Ok(())
}

async fn ensure_user(db: &DbHandle, email: &str, name: &str, password: &str) -> Result<i64> {
    if let Some(u) = find_user_by_email(db, email).await? {
        return Ok(u.id);
    }
    let u = register_user(db, email, name, password).await?;
    Ok(u.id)
}
