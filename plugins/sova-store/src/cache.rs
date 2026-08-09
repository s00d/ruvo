//! Thin JSON cache helper over any [`KvStore`] (Memory / File / Sql / Redis).

use crate::{namespace, AppStore, KvStore};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
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

fn trunc(key: &str) -> String {
    const MAX: usize = 120;
    if key.len() <= MAX {
        key.to_string()
    } else {
        format!("{}…", &key[..MAX - 1])
    }
}

fn rid() -> Option<String> {
    sova_core::current_request_id()
}

impl Cache {
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self { store }
    }

    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let started = Instant::now();
        let bytes = self.store.get(key).await;
        let hit = bytes.is_some();
        let n = bytes.as_ref().map(|b| b.len() as u64);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(
            target: "sova.store",
            op = "get",
            backend = "cache",
            key = %trunc(key),
            hit,
            bytes = n,
            duration_ms = ms,
            request_id = rid().as_deref().unwrap_or(""),
            "sova.store"
        );
        let bytes = bytes?;
        serde_json::from_slice(&bytes).ok()
    }

    pub async fn set_json<T: Serialize>(
        &self,
        key: &str,
        val: &T,
        ttl: Option<Duration>,
    ) -> Result<(), CacheError> {
        let started = Instant::now();
        let raw = serde_json::to_vec(val).map_err(|e| CacheError::Serialize(e.to_string()))?;
        let n = raw.len() as u64;
        self.store.set(key, Bytes::from(raw), ttl).await;
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(
            target: "sova.store",
            op = "set",
            backend = "cache",
            key = %trunc(key),
            bytes = n,
            duration_ms = ms,
            request_id = rid().as_deref().unwrap_or(""),
            "sova.store"
        );
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
        let started = Instant::now();
        self.store.remove(key).await;
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(
            target: "sova.store",
            op = "remove",
            backend = "cache",
            key = %trunc(key),
            duration_ms = ms,
            request_id = rid().as_deref().unwrap_or(""),
            "sova.store"
        );
    }
}

impl AppStore {
    /// Namespaced JSON cache (`cache:` prefix on the shared backend).
    pub fn cache(&self) -> Cache {
        Cache::new(Arc::new(namespace(Arc::clone(&self.inner), "cache")))
    }
}
