//! In-process positive membership cache for AuthorizeEngine.

use crate::types::{Membership, ScopeRef};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type CacheKey = (String, i64, i64);

#[derive(Clone, Default)]
pub struct MembershipCache {
    inner: Arc<Mutex<HashMap<CacheKey, Membership>>>,
}

impl MembershipCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(scope: &ScopeRef, user_id: i64) -> CacheKey {
        (scope.kind.clone(), scope.id, user_id)
    }

    pub fn get(&self, scope: &ScopeRef, user_id: i64) -> Option<Membership> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&Self::key(scope, user_id))
            .cloned()
    }

    pub fn put(&self, membership: Membership) {
        let key = (
            membership.scope_kind.clone(),
            membership.scope_id,
            membership.user_id,
        );
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, membership);
    }

    pub fn invalidate(&self, scope: &ScopeRef, user_id: i64) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&Self::key(scope, user_id));
    }

    pub fn invalidate_scope(&self, scope: &ScopeRef) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.retain(|(kind, id, _), _| !(kind == &scope.kind && *id == scope.id));
    }

    pub fn clear(&self) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
}

/// Options for membership mutations (audit actor + cache invalidate).
#[derive(Clone, Copy, Default)]
pub struct MutateOpts<'a> {
    pub actor_id: Option<i64>,
    pub cache: Option<&'a MembershipCache>,
}

impl<'a> MutateOpts<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn actor(mut self, actor_id: i64) -> Self {
        self.actor_id = Some(actor_id);
        self
    }

    pub fn cache(mut self, cache: &'a MembershipCache) -> Self {
        self.cache = Some(cache);
        self
    }
}
