//! Authorization engine.

use crate::cache::MembershipCache;
use crate::matrix::RoleMatrix;
use crate::membership::MembershipStore;
use crate::owner::OwnerRegistry;
use crate::types::{
    ability_error, is_privileged_role, Ability, Capabilities, Membership, ResourceRef, ScopeRef,
    ROLE_OWNER,
};
use sova_core::{Error, Result};
use sova_db::DbHandle;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AuthorizeEngine {
    matrices: HashMap<&'static str, RoleMatrix>,
    owners: OwnerRegistry,
    membership_cache: MembershipCache,
}

impl AuthorizeEngine {
    pub fn new(
        matrices: HashMap<&'static str, RoleMatrix>,
        owners: OwnerRegistry,
    ) -> Self {
        Self {
            matrices,
            owners,
            membership_cache: MembershipCache::new(),
        }
    }

    pub fn membership_cache(&self) -> &MembershipCache {
        &self.membership_cache
    }

    pub fn matrix(&self, scope_kind: &str) -> Option<&RoleMatrix> {
        self.matrices.get(scope_kind)
    }

    pub async fn membership(
        &self,
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
    ) -> Result<Option<Membership>> {
        if let Some(hit) = self.membership_cache.get(&scope, user_id) {
            return Ok(Some(hit));
        }
        let found = MembershipStore::find(db, scope.clone(), user_id).await?;
        if let Some(ref m) = found {
            self.membership_cache.put(m.clone());
        }
        Ok(found)
    }

    pub async fn require_member(
        &self,
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
    ) -> Result<Membership> {
        self.membership(db, scope, user_id)
            .await?
            .ok_or(Error::NotFound)
    }

    pub async fn require_privileged(
        &self,
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
    ) -> Result<Membership> {
        let m = self.require_member(db, scope, user_id).await?;
        if !is_privileged_role(&m.role) {
            return Err(Error::custom(403, "scope admin required"));
        }
        Ok(m)
    }

    pub async fn require_owner_role(
        &self,
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
    ) -> Result<Membership> {
        let m = self.require_member(db, scope, user_id).await?;
        if m.role != ROLE_OWNER {
            return Err(Error::custom(403, "scope owner required"));
        }
        Ok(m)
    }

    pub async fn is_privileged(
        &self,
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
    ) -> Result<bool> {
        Ok(self
            .membership(db, scope, user_id)
            .await?
            .is_some_and(|m| is_privileged_role(&m.role)))
    }

    pub async fn effective(
        &self,
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
        resource: Option<ResourceRef>,
    ) -> Result<Capabilities> {
        let m = self.require_member(db, scope.clone(), user_id).await?;
        let matrix = self
            .matrices
            .get(scope.kind.as_str())
            .ok_or_else(|| Error::Internal(format!("no matrix for scope kind {}", scope.kind)))?;
        let is_owner = self.owners.is_owner(db, resource, user_id).await?;
        Ok(matrix.capabilities(&m.role, resource, is_owner))
    }

    pub async fn authorize(
        &self,
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
        ability: Ability,
        resource: Option<ResourceRef>,
    ) -> Result<Capabilities> {
        let caps = self.effective(db, scope, user_id, resource).await?;
        if !caps.has(ability) {
            return Err(ability_error(ability));
        }
        Ok(caps)
    }

    pub async fn can(
        &self,
        db: &DbHandle,
        scope: ScopeRef,
        user_id: i64,
        ability: Ability,
        resource: Option<ResourceRef>,
    ) -> Result<bool> {
        Ok(self
            .effective(db, scope, user_id, resource)
            .await
            .map(|c| c.has(ability))
            .unwrap_or(false))
    }

    pub fn owners(&self) -> &OwnerRegistry {
        &self.owners
    }
}

pub type SharedEngine = Arc<AuthorizeEngine>;
