//! In-memory [`BlobStore`] (tests / ephemeral).

use crate::{normalize_key, normalize_prefix, BlobStore, BoxFuture, PutOpts, StorageError};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, Default)]
pub struct MemoryStore {
    inner: Arc<Mutex<HashMap<String, Bytes>>>,
}

fn lock_map(inner: &Mutex<HashMap<String, Bytes>>) -> MutexGuard<'_, HashMap<String, Bytes>> {
    match inner.try_lock() {
        Ok(g) => g,
        Err(_) => tokio::task::block_in_place(|| inner.lock().unwrap()),
    }
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BlobStore for MemoryStore {
    fn put<'a>(
        &'a self,
        key: &'a str,
        data: Bytes,
        _opts: PutOpts,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            lock_map(&self.inner).insert(key, data);
            Ok(())
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<Bytes>, StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            Ok(lock_map(&self.inner).get(&key).cloned())
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            lock_map(&self.inner).remove(&key);
            Ok(())
        })
    }

    fn exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            Ok(lock_map(&self.inner).contains_key(&key))
        })
    }

    fn list<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, Result<Vec<String>, StorageError>> {
        Box::pin(async move {
            let prefix = normalize_prefix(prefix)?;
            let map = lock_map(&self.inner);
            let mut keys: Vec<String> = map
                .keys()
                .filter(|k| {
                    if prefix.is_empty() {
                        true
                    } else if prefix.ends_with('/') {
                        k.starts_with(&prefix)
                    } else {
                        *k == &prefix || k.starts_with(&format!("{prefix}/"))
                    }
                })
                .cloned()
                .collect();
            keys.sort();
            Ok(keys)
        })
    }
}
