//! In-memory [`BlobStore`] (tests / ephemeral).

use crate::{normalize_key, normalize_prefix, BlobStore, BoxFuture, PutOpts, StorageError};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct MemoryStore {
    inner: Arc<Mutex<HashMap<String, Bytes>>>,
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
            self.inner.lock().unwrap().insert(key, data);
            Ok(())
        })
    }

    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<Bytes>, StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            Ok(self.inner.lock().unwrap().get(&key).cloned())
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            self.inner.lock().unwrap().remove(&key);
            Ok(())
        })
    }

    fn exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, StorageError>> {
        Box::pin(async move {
            let key = normalize_key(key)?;
            Ok(self.inner.lock().unwrap().contains_key(&key))
        })
    }

    fn list<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, Result<Vec<String>, StorageError>> {
        Box::pin(async move {
            let prefix = normalize_prefix(prefix)?;
            let map = self.inner.lock().unwrap();
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
