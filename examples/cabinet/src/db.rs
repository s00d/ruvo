use crate::entity::note;
use chrono::{FixedOffset, Utc};
use ruvo::{ActiveModelTrait, ColumnTrait, DbHandle, EntityTrait, Error, Result, Set};
use sea_orm::{QueryFilter, QueryOrder, QuerySelect};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Note {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub body: String,
    pub created_at: String,
}

fn map_note(m: note::Model) -> Note {
    Note {
        id: m.id,
        user_id: m.user_id,
        title: m.title,
        body: m.body,
        created_at: m.created_at.to_rfc3339(),
    }
}

pub async fn list_notes(db: &DbHandle, user_id: i64, limit: u64) -> Result<Vec<Note>> {
    let rows = note::Entity::find()
        .filter(note::Column::UserId.eq(user_id))
        .order_by_desc(note::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(rows.into_iter().map(map_note).collect())
}

pub async fn create_note(db: &DbHandle, user_id: i64, title: &str, body: &str) -> Result<i64> {
    let now: chrono::DateTime<FixedOffset> = Utc::now().into();
    let row = note::ActiveModel {
        id: Default::default(),
        user_id: Set(user_id),
        title: Set(title.into()),
        body: Set(body.into()),
        created_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(row.id)
}

pub async fn delete_note(db: &DbHandle, user_id: i64, note_id: i64) -> Result<bool> {
    let res = note::Entity::delete_many()
        .filter(note::Column::Id.eq(note_id))
        .filter(note::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(res.rows_affected > 0)
}
