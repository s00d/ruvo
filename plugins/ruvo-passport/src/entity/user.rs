use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub email: String,
    pub password_hash: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::refresh_token::Entity")]
    RefreshTokens,
    #[cfg(feature = "oauth")]
    #[sea_orm(has_many = "super::oauth_account::Entity")]
    OauthAccounts,
}

impl Related<super::refresh_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RefreshTokens.def()
    }
}

#[cfg(feature = "oauth")]
impl Related<super::oauth_account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OauthAccounts.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
