use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub email: String,
    pub password_hash: Option<String>,
    pub name: String,
    pub avatar_path: Option<String>,
    pub email_verified_at: Option<DateTimeUtc>,
    pub two_factor_secret: Option<String>,
    pub two_factor_recovery_codes: Option<String>,
    pub two_factor_confirmed_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
