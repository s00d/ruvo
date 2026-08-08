//! SeaORM entity for `activity_log`.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "activity_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub actor_id: Option<i64>,
    pub subject_type: String,
    pub subject_id: String,
    pub event: String,
    /// JSON object (no secrets).
    pub properties: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
