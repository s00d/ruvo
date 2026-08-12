//! Scope membership queries.

use crate::audit::{AuditStore, AUDIT_ADDED, AUDIT_REMOVED, AUDIT_ROLE_CHANGED};
use crate::cache::MutateOpts;
use crate::entity::{scope_membership, ScopeMembership};
use crate::types::{Membership, ScopeRef};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sova_core::Result;
use sova_db::{ActiveModelTrait, DbError, DbHandle, Set};

pub struct MembershipStore;

impl MembershipStore {
    pub async fn find(
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
    ) -> Result<Option<Membership>> {
        Ok(ScopeMembership::find()
            .filter(scope_membership::Column::ScopeKind.eq(scope.kind))
            .filter(scope_membership::Column::ScopeId.eq(scope.id))
            .filter(scope_membership::Column::UserId.eq(user_id))
            .one(db)
            .await
            .map_err(DbError::from)?
            .map(model_to_membership))
    }

    pub async fn require(
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
    ) -> Result<Membership> {
        Self::find(db, scope, user_id)
            .await?
            .ok_or(sova_core::Error::NotFound)
    }

    pub async fn list_for_scope(
        db: &DbHandle,
        scope: ScopeRef,
    ) -> Result<Vec<Membership>> {
        Ok(ScopeMembership::find()
            .filter(scope_membership::Column::ScopeKind.eq(scope.kind))
            .filter(scope_membership::Column::ScopeId.eq(scope.id))
            .all(db)
            .await
            .map_err(DbError::from)?
            .into_iter()
            .map(model_to_membership)
            .collect())
    }

    pub async fn list_for_user(
        db: &DbHandle,
        scope_kind: &str,
        user_id: i64,
    ) -> Result<Vec<Membership>> {
        Ok(ScopeMembership::find()
            .filter(scope_membership::Column::ScopeKind.eq(scope_kind))
            .filter(scope_membership::Column::UserId.eq(user_id))
            .all(db)
            .await
            .map_err(DbError::from)?
            .into_iter()
            .map(model_to_membership)
            .collect())
    }

    pub async fn add(
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
        role: &str,
    ) -> Result<Membership> {
        Self::add_with(db, scope, user_id, role, MutateOpts::default()).await
    }

    pub async fn add_with(
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
        role: &str,
        opts: MutateOpts<'_>,
    ) -> Result<Membership> {
        let m = model_to_membership(
            scope_membership::ActiveModel {
                scope_kind: Set(scope.kind.clone()),
                scope_id: Set(scope.id),
                user_id: Set(user_id),
                role: Set(role.into()),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(DbError::from)?,
        );
        let _ = AuditStore::record(
            db,
            &scope,
            user_id,
            opts.actor_id,
            AUDIT_ADDED,
            None,
            Some(role),
        )
        .await;
        if let Some(cache) = opts.cache {
            cache.invalidate(&scope, user_id);
            cache.put(m.clone());
        }
        Ok(m)
    }

    pub async fn update_role(
        db: &DbHandle,
        membership_id: i64,
        role: &str,
    ) -> Result<Membership> {
        Self::update_role_with(db, membership_id, role, MutateOpts::default()).await
    }

    pub async fn update_role_with(
        db: &DbHandle,
        membership_id: i64,
        role: &str,
        opts: MutateOpts<'_>,
    ) -> Result<Membership> {
        let row = ScopeMembership::find_by_id(membership_id)
            .one(db)
            .await
            .map_err(DbError::from)?
            .ok_or(sova_core::Error::NotFound)?;
        let old_role = row.role.clone();
        let scope = ScopeRef::new(row.scope_kind.clone(), row.scope_id);
        let user_id = row.user_id;
        let mut am: scope_membership::ActiveModel = row.into();
        am.role = Set(role.into());
        let m = model_to_membership(am.update(db).await.map_err(DbError::from)?);
        let _ = AuditStore::record(
            db,
            &scope,
            user_id,
            opts.actor_id,
            AUDIT_ROLE_CHANGED,
            Some(&old_role),
            Some(role),
        )
        .await;
        if let Some(cache) = opts.cache {
            cache.invalidate(&scope, user_id);
            cache.put(m.clone());
        }
        Ok(m)
    }

    pub async fn remove(db: &DbHandle, membership_id: i64) -> Result<()> {
        Self::remove_with(db, membership_id, MutateOpts::default()).await
    }

    pub async fn remove_with(
        db: &DbHandle,
        membership_id: i64,
        opts: MutateOpts<'_>,
    ) -> Result<()> {
        let row = ScopeMembership::find_by_id(membership_id)
            .one(db)
            .await
            .map_err(DbError::from)?
            .ok_or(sova_core::Error::NotFound)?;
        let scope = ScopeRef::new(row.scope_kind.clone(), row.scope_id);
        let user_id = row.user_id;
        let old_role = row.role.clone();
        ScopeMembership::delete_by_id(membership_id)
            .exec(db)
            .await
            .map_err(DbError::from)?;
        let _ = AuditStore::record(
            db,
            &scope,
            user_id,
            opts.actor_id,
            AUDIT_REMOVED,
            Some(&old_role),
            None,
        )
        .await;
        if let Some(cache) = opts.cache {
            cache.invalidate(&scope, user_id);
        }
        Ok(())
    }

    pub async fn count_role(
        db: &DbHandle,
        scope: ScopeRef,
        role: &str,
    ) -> Result<usize> {
        Ok(ScopeMembership::find()
            .filter(scope_membership::Column::ScopeKind.eq(scope.kind))
            .filter(scope_membership::Column::ScopeId.eq(scope.id))
            .filter(scope_membership::Column::Role.eq(role))
            .all(db)
            .await
            .map_err(DbError::from)?
            .len())
    }
}

fn model_to_membership(m: scope_membership::Model) -> Membership {
    Membership {
        id: m.id,
        scope_kind: m.scope_kind,
        scope_id: m.scope_id,
        user_id: m.user_id,
        role: m.role,
    }
}
