use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub mod scope_membership {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "scope_memberships")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub scope_kind: String,
        pub scope_id: i64,
        pub user_id: i64,
        pub role: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

#[cfg(feature = "invites")]
pub mod scope_invitation {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "scope_invitations")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub scope_kind: String,
        pub scope_id: i64,
        pub email: String,
        pub role: String,
        pub invited_by: i64,
        pub token: String,
        pub status: String,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod scope_membership_audit {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "scope_membership_audit")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub scope_kind: String,
        pub scope_id: i64,
        pub user_id: i64,
        pub actor_id: Option<i64>,
        pub action: String,
        pub old_role: Option<String>,
        pub new_role: Option<String>,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub use scope_membership::Entity as ScopeMembership;
pub use scope_membership_audit::Entity as ScopeMembershipAudit;

#[cfg(feature = "invites")]
pub use scope_invitation::Entity as ScopeInvitation;
