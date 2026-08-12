//! Request extensions for scoped authorization.

use crate::engine::SharedEngine;
use crate::plugin::ScopeAuthState;
use crate::types::{Ability, Capabilities, Membership, ResourceRef, ScopeRef};
use sova_core::{Error, Request, Result};
use sova_db::DbExt;

pub trait ScopeExt {
    fn scope_engine(&self) -> Result<SharedEngine>;
    fn scope_authorize(
        &self,
        scope: ScopeRef,
        user_id: i64,
        ability: Ability,
        resource: Option<ResourceRef>,
    ) -> impl std::future::Future<Output = Result<Capabilities>> + Send;
    fn scope_effective(
        &self,
        scope: ScopeRef,
        user_id: i64,
        resource: Option<ResourceRef>,
    ) -> impl std::future::Future<Output = Result<Capabilities>> + Send;
    fn scope_membership(
        &self,
        scope: ScopeRef,
        user_id: i64,
    ) -> impl std::future::Future<Output = Result<Membership>> + Send;
}

impl ScopeExt for Request {
    fn scope_engine(&self) -> Result<SharedEngine> {
        self.try_state::<ScopeAuthState>()
            .map(|s| s.engine.clone())
            .ok_or_else(|| Error::Internal("ScopeAuth plugin not installed".into()))
    }

    async fn scope_authorize(
        &self,
        scope: ScopeRef,
        user_id: i64,
        ability: Ability,
        resource: Option<ResourceRef>,
    ) -> Result<Capabilities> {
        let engine = self.scope_engine()?;
        let db = self.db().clone();
        engine.authorize(&db, scope, user_id, ability, resource).await
    }

    async fn scope_effective(
        &self,
        scope: ScopeRef,
        user_id: i64,
        resource: Option<ResourceRef>,
    ) -> Result<Capabilities> {
        let engine = self.scope_engine()?;
        let db = self.db().clone();
        engine.effective(&db, scope, user_id, resource).await
    }

    async fn scope_membership(&self, scope: ScopeRef, user_id: i64) -> Result<Membership> {
        let engine = self.scope_engine()?;
        let db = self.db().clone();
        engine.require_member(&db, scope, user_id).await
    }
}
