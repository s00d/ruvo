//! Scope membership role-change audit log.

use crate::entity::scope_membership_audit;
use crate::types::ScopeRef;
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use sova_core::Result;
use sova_db::{ActiveModelTrait, DbError, DbHandle, Set};

pub const AUDIT_ADDED: &str = "added";
pub const AUDIT_ROLE_CHANGED: &str = "role_changed";
pub const AUDIT_REMOVED: &str = "removed";

#[derive(Clone, Debug)]
pub struct AuditEntry {
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

pub struct AuditStore;

impl AuditStore {
    pub async fn record(
        db: &DbHandle,
        scope: &ScopeRef,
        user_id: i64,
        actor_id: Option<i64>,
        action: &str,
        old_role: Option<&str>,
        new_role: Option<&str>,
    ) -> Result<AuditEntry> {
        let row = scope_membership_audit::ActiveModel {
            scope_kind: Set(scope.kind.clone()),
            scope_id: Set(scope.id),
            user_id: Set(user_id),
            actor_id: Set(actor_id),
            action: Set(action.into()),
            old_role: Set(old_role.map(str::to_string)),
            new_role: Set(new_role.map(str::to_string)),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(DbError::from)?;
        Ok(model_to_entry(row))
    }

    pub async fn list_for_scope(
        db: &DbHandle,
        scope: ScopeRef,
        limit: u64,
    ) -> Result<Vec<AuditEntry>> {
        Ok(scope_membership_audit::Entity::find()
            .filter(scope_membership_audit::Column::ScopeKind.eq(scope.kind))
            .filter(scope_membership_audit::Column::ScopeId.eq(scope.id))
            .order_by_desc(scope_membership_audit::Column::CreatedAt)
            .limit(limit)
            .all(db)
            .await
            .map_err(DbError::from)?
            .into_iter()
            .map(model_to_entry)
            .collect())
    }

    pub async fn list_for_user(
        db: &DbHandle,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<AuditEntry>> {
        Ok(scope_membership_audit::Entity::find()
            .filter(scope_membership_audit::Column::UserId.eq(user_id))
            .order_by_desc(scope_membership_audit::Column::CreatedAt)
            .limit(limit)
            .all(db)
            .await
            .map_err(DbError::from)?
            .into_iter()
            .map(model_to_entry)
            .collect())
    }
}

fn model_to_entry(m: scope_membership_audit::Model) -> AuditEntry {
    AuditEntry {
        id: m.id,
        scope_kind: m.scope_kind,
        scope_id: m.scope_id,
        user_id: m.user_id,
        actor_id: m.actor_id,
        action: m.action,
        old_role: m.old_role,
        new_role: m.new_role,
        created_at: m.created_at,
    }
}
