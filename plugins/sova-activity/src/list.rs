//! Query helpers for activity_log.

use crate::entity;
use chrono::{DateTime, Utc};
use sova_core::Result;
use sova_db::{DbError, DbHandle};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Serialize;

#[derive(Debug, Clone, Default)]
pub struct ActivityFilter {
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub event: Option<String>,
    pub actor_id: Option<i64>,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityRow {
    pub id: i64,
    pub actor_id: Option<i64>,
    pub subject_type: String,
    pub subject_id: String,
    pub event: String,
    pub properties: serde_json::Value,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<entity::Model> for ActivityRow {
    fn from(m: entity::Model) -> Self {
        let properties = serde_json::from_str(&m.properties).unwrap_or(serde_json::json!({}));
        Self {
            id: m.id,
            actor_id: m.actor_id,
            subject_type: m.subject_type,
            subject_id: m.subject_id,
            event: m.event,
            properties,
            ip: m.ip,
            user_agent: m.user_agent,
            created_at: m.created_at,
        }
    }
}

pub async fn list_activity(db: &DbHandle, filter: ActivityFilter) -> Result<Vec<ActivityRow>> {
    let limit = filter.limit.clamp(1, 200);
    let mut q = entity::Entity::find();
    if let Some(ref t) = filter.subject_type {
        q = q.filter(entity::Column::SubjectType.eq(t.clone()));
    }
    if let Some(ref id) = filter.subject_id {
        q = q.filter(entity::Column::SubjectId.eq(id.clone()));
    }
    if let Some(ref ev) = filter.event {
        q = q.filter(entity::Column::Event.eq(ev.clone()));
    }
    if let Some(aid) = filter.actor_id {
        q = q.filter(entity::Column::ActorId.eq(aid));
    }
    let rows = q
        .order_by_desc(entity::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(|e| sova_core::Error::from(DbError::from(e)))?;
    Ok(rows.into_iter().map(ActivityRow::from).collect())
}
