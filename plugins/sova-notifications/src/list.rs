//! Query helpers for notifications inbox.

use crate::entity;
use chrono::{DateTime, Utc};
use sova_core::Result;
use sova_db::{DbError, DbHandle};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::Serialize;

#[derive(Debug, Clone, Default)]
pub struct NotificationFilter {
    pub channel: Option<String>,
    pub unread_only: bool,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationRow {
    pub id: i64,
    pub user_id: i64,
    pub channel: String,
    pub event: String,
    pub title: String,
    pub body: Option<String>,
    pub data: serde_json::Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<entity::Model> for NotificationRow {
    fn from(m: entity::Model) -> Self {
        let data = serde_json::from_str(&m.data).unwrap_or(serde_json::json!({}));
        Self {
            id: m.id,
            user_id: m.user_id,
            channel: m.channel,
            event: m.event,
            title: m.title,
            body: m.body,
            data,
            read_at: m.read_at,
            created_at: m.created_at,
        }
    }
}

pub async fn list_notifications(
    db: &DbHandle,
    user_id: i64,
    filter: NotificationFilter,
) -> Result<Vec<NotificationRow>> {
    let limit = filter.limit.clamp(1, 200);
    let mut q = entity::Entity::find().filter(entity::Column::UserId.eq(user_id));
    if let Some(ref ch) = filter.channel {
        q = q.filter(entity::Column::Channel.eq(ch.clone()));
    }
    if filter.unread_only {
        q = q.filter(entity::Column::ReadAt.is_null());
    }
    let rows = q
        .order_by_desc(entity::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(|e| sova_core::Error::from(DbError::from(e)))?;
    Ok(rows.into_iter().map(NotificationRow::from).collect())
}

pub async fn unread_count(db: &DbHandle, user_id: i64, channel: Option<&str>) -> Result<u64> {
    let mut q = entity::Entity::find()
        .filter(entity::Column::UserId.eq(user_id))
        .filter(entity::Column::ReadAt.is_null());
    if let Some(ch) = channel {
        q = q.filter(entity::Column::Channel.eq(ch));
    }
    q.count(db)
        .await
        .map_err(|e| sova_core::Error::from(DbError::from(e)))
}

pub async fn mark_read(db: &DbHandle, user_id: i64, id: i64) -> Result<bool> {
    let Some(row) = entity::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| sova_core::Error::from(DbError::from(e)))?
    else {
        return Ok(false);
    };
    if row.user_id != user_id {
        return Ok(false);
    }
    if row.read_at.is_some() {
        return Ok(true);
    }
    let mut am: entity::ActiveModel = row.into();
    am.read_at = Set(Some(Utc::now()));
    am.update(db)
        .await
        .map_err(|e| sova_core::Error::from(DbError::from(e)))?;
    Ok(true)
}

pub async fn mark_all_read(db: &DbHandle, user_id: i64, channel: Option<&str>) -> Result<u64> {
    let mut q = entity::Entity::find()
        .filter(entity::Column::UserId.eq(user_id))
        .filter(entity::Column::ReadAt.is_null());
    if let Some(ch) = channel {
        q = q.filter(entity::Column::Channel.eq(ch));
    }
    let rows = q
        .all(db)
        .await
        .map_err(|e| sova_core::Error::from(DbError::from(e)))?;
    let n = rows.len() as u64;
    let now = Utc::now();
    for row in rows {
        let mut am: entity::ActiveModel = row.into();
        am.read_at = Set(Some(now));
        am.update(db)
            .await
            .map_err(|e| sova_core::Error::from(DbError::from(e)))?;
    }
    Ok(n)
}
