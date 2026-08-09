//! Shared query helpers for the HN demo.

use crate::entity::{comment, story, vote};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use sova::{
    find_user_by_id, ActiveModelTrait, DbError, DbHandle, Result, Set,
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct StoryRow {
    pub id: i64,
    pub title: String,
    pub url: Option<String>,
    pub text: Option<String>,
    pub points: i32,
    pub user_id: i64,
    pub author: String,
    pub created_at: String,
    pub comment_count: u64,
}

pub async fn list_stories(db: &DbHandle, newest: bool, limit: u64) -> Result<Vec<StoryRow>> {
    let mut q = story::Entity::find();
    q = if newest {
        q.order_by_desc(story::Column::CreatedAt)
    } else {
        q.order_by_desc(story::Column::Points)
            .order_by_desc(story::Column::CreatedAt)
    };
    let rows = q.limit(limit).all(db).await.map_err(DbError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for s in rows {
        out.push(enrich_story(db, s).await?);
    }
    Ok(out)
}

pub async fn get_story(db: &DbHandle, id: i64) -> Result<Option<StoryRow>> {
    let Some(s) = story::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(DbError::from)?
    else {
        return Ok(None);
    };
    Ok(Some(enrich_story(db, s).await?))
}

async fn enrich_story(db: &DbHandle, s: story::Model) -> Result<StoryRow> {
    let author = author_name(db, s.user_id).await?;
    let comment_count = comment::Entity::find()
        .filter(comment::Column::StoryId.eq(s.id))
        .count(db)
        .await
        .map_err(DbError::from)?;
    Ok(StoryRow {
        id: s.id,
        title: s.title,
        url: s.url,
        text: s.text,
        points: s.points,
        user_id: s.user_id,
        author,
        created_at: s.created_at.to_rfc3339(),
        comment_count,
    })
}

pub async fn author_name(db: &DbHandle, user_id: i64) -> Result<String> {
    if let Some(u) = find_user_by_id(db, user_id).await? {
        return Ok(u.name);
    }
    Ok(format!("user#{user_id}"))
}

pub async fn create_story(
    db: &DbHandle,
    user_id: i64,
    title: String,
    url: Option<String>,
    text: Option<String>,
) -> Result<story::Model> {
    let row = story::ActiveModel {
        user_id: Set(user_id),
        title: Set(title),
        url: Set(url),
        text: Set(text),
        points: Set(1),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(DbError::from)?;
    Ok(row)
}

pub async fn upvote(db: &DbHandle, user_id: i64, story_id: i64) -> Result<bool> {
    let existing = vote::Entity::find()
        .filter(vote::Column::UserId.eq(user_id))
        .filter(vote::Column::StoryId.eq(story_id))
        .one(db)
        .await
        .map_err(DbError::from)?;
    if existing.is_some() {
        return Ok(false);
    }
    vote::ActiveModel {
        user_id: Set(user_id),
        story_id: Set(story_id),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(DbError::from)?;

    if let Some(s) = story::Entity::find_by_id(story_id)
        .one(db)
        .await
        .map_err(DbError::from)?
    {
        let pts = s.points + 1;
        let mut am: story::ActiveModel = s.into();
        am.points = Set(pts);
        am.update(db).await.map_err(DbError::from)?;
    }
    Ok(true)
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CommentRow {
    pub id: i64,
    pub body: String,
    pub user_id: i64,
    pub author: String,
    pub created_at: String,
}

pub async fn list_comments(db: &DbHandle, story_id: i64) -> Result<Vec<CommentRow>> {
    let rows = comment::Entity::find()
        .filter(comment::Column::StoryId.eq(story_id))
        .order_by_asc(comment::Column::CreatedAt)
        .all(db)
        .await
        .map_err(DbError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for c in rows {
        out.push(CommentRow {
            id: c.id,
            body: c.body,
            user_id: c.user_id,
            author: author_name(db, c.user_id).await?,
            created_at: c.created_at.to_rfc3339(),
        });
    }
    Ok(out)
}

pub async fn add_comment(
    db: &DbHandle,
    user_id: i64,
    story_id: i64,
    body: String,
) -> Result<comment::Model> {
    let row = comment::ActiveModel {
        user_id: Set(user_id),
        story_id: Set(story_id),
        body: Set(body),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(DbError::from)?;
    Ok(row)
}
