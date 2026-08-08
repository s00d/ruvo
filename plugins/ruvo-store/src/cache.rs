//! Thin JSON cache helper over any [`KvStore`] (Memory / File / Sql / Redis).

use crate::{namespace, AppStore, KvStore};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("serialize: {0}")]
    Serialize(String),
    #[error("deserialize: {0}")]
    Deserialize(String),
    #[error("{0}")]
    Msg(String),
}

/// JSON get/set/remember on a [`KvStore`].
#[derive(Clone)]
pub struct Cache {
    store: Arc<dyn KvStore>,
}

impl Cache {
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self { store }
    }

    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let bytes = self.store.get(key).await?;
        serde_json::from_slice(&bytes).ok()
    }

    pub async fn set_json<T: Serialize>(
        &self,
        key: &str,
        val: &T,
        ttl: Option<Duration>,
    ) -> Result<(), CacheError> {
        let bytes = serde_json::to_vec(val).map_err(|e| CacheError::Serialize(e.to_string()))?;
        self.store.set(key, Bytes::from(bytes), ttl).await;
        Ok(())
    }

    /// Return cached value or compute, store, and return.
    pub async fn remember<T, F, Fut>(
        &self,
        key: &str,
        ttl: Option<Duration>,
        f: F,
    ) -> Result<T, CacheError>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, CacheError>>,
    {
        if let Some(hit) = self.get_json::<T>(key).await {
            return Ok(hit);
        }
        let val = f().await?;
        self.set_json(key, &val, ttl).await?;
        Ok(val)
    }

    pub async fn invalidate(&self, key: &str) {
        self.store.remove(key).await;
    }
}

impl AppStore {
    /// Namespaced JSON cache (`cache:` prefix on the shared backend).
    pub fn cache(&self) -> Cache {
        Cache::new(Arc::new(namespace(Arc::clone(&self.inner), "cache")))
    }
}
