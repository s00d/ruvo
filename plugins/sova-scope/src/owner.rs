//! Resource ownership resolvers.

use async_trait::async_trait;
use sova_db::DbHandle;
use sova_core::Result;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

#[async_trait]
pub trait OwnerResolver: Send + Sync {
    async fn owner_user_id(&self, db: &DbHandle, resource_id: i64) -> Result<Option<i64>>;
}

#[derive(Default)]
pub struct OwnerRegistry {
    resolvers: HashMap<&'static str, Arc<dyn OwnerResolver>>,
}

impl OwnerRegistry {
    pub fn register(&mut self, kind: &'static str, resolver: Arc<dyn OwnerResolver>) {
        self.resolvers.insert(kind, resolver);
    }

    pub async fn is_owner(
        &self,
        db: &DbHandle,
        resource: Option<crate::types::ResourceRef>,
        user_id: i64,
    ) -> Result<bool> {
        let Some(resource) = resource else {
            return Ok(false);
        };
        let Some(resolver) = self.resolvers.get(resource.kind) else {
            return Ok(false);
        };
        Ok(resolver
            .owner_user_id(db, resource.id)
            .await?
            .is_some_and(|id| id == user_id))
    }
}

struct CallbackOwner<F>(F);

#[async_trait]
impl<F, Fut> OwnerResolver for CallbackOwner<F>
where
    F: Fn(DbHandle, i64) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Option<i64>>> + Send,
{
    async fn owner_user_id(&self, db: &DbHandle, resource_id: i64) -> Result<Option<i64>> {
        (self.0)(db.clone(), resource_id).await
    }
}

pub struct FnOwnerResolver<F>(F);

impl<F> FnOwnerResolver<F> {
    pub fn new(f: F) -> Self {
        Self(f)
    }
}

impl<F, Fut> OwnerResolver for FnOwnerResolver<F>
where
    F: Fn(DbHandle, i64) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Option<i64>>> + Send,
{
    fn owner_user_id<'life0, 'life1, 'async_trait>(
        &'life0 self,
        db: &'life1 DbHandle,
        resource_id: i64,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<Option<i64>>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
    {
        Box::pin(async move { (self.0)(db.clone(), resource_id).await })
    }
}

pub fn callback_owner<F, Fut>(f: F) -> Arc<dyn OwnerResolver>
where
    F: Fn(DbHandle, i64) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Option<i64>>> + Send + 'static,
{
    Arc::new(CallbackOwner(f))
}
