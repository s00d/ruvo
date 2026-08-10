//! Embedded [`KvStore`] on [redb](https://crates.io/crates/redb) (ACID, process-local).

use crate::{BoxFuture, KvStore};
use bytes::Bytes;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sova_kv");

/// File-backed KvStore via redb. Survives restart; no network daemon.
#[derive(Clone)]
pub struct RedbStore {
    db: Arc<Database>,
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Wire format: 8-byte LE expiry unix-ms (`0` = no TTL) + payload.
fn pack(val: &[u8], ttl: Option<Duration>) -> Vec<u8> {
    let exp = ttl
        .map(|d| now_ms().saturating_add(d.as_millis() as u64))
        .unwrap_or(0);
    let mut out = Vec::with_capacity(8 + val.len());
    out.extend_from_slice(&exp.to_le_bytes());
    out.extend_from_slice(val);
    out
}

fn unpack(raw: &[u8]) -> Option<Bytes> {
    if raw.len() < 8 {
        return None;
    }
    let exp = u64::from_le_bytes(raw[0..8].try_into().ok()?);
    if exp != 0 && exp <= now_ms() {
        return None;
    }
    Some(Bytes::copy_from_slice(&raw[8..]))
}

fn is_expired(raw: &[u8]) -> bool {
    if raw.len() < 8 {
        return true;
    }
    let exp = u64::from_le_bytes(raw[0..8].try_into().unwrap_or([0; 8]));
    exp != 0 && exp <= now_ms()
}

impl RedbStore {
    /// Open or create a database at `path` (creates parent dirs).
    pub fn open(path: impl AsRef<Path>) -> Result<Self, redb::DatabaseError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        Ok(Self {
            db: Arc::new(Database::create(path)?),
        })
    }

    fn get_sync(&self, key: &str) -> Option<Bytes> {
        let read = self.db.begin_read().ok()?;
        let table = read.open_table(TABLE).ok()?;
        let guard = table.get(key).ok()??;
        let raw = guard.value();
        if is_expired(raw) {
            drop(guard);
            drop(table);
            drop(read);
            let _ = self.remove_sync(key);
            return None;
        }
        unpack(raw)
    }

    fn set_sync(&self, key: &str, val: Bytes, ttl: Option<Duration>) -> bool {
        let Ok(txn) = self.db.begin_write() else {
            return false;
        };
        {
            let Ok(mut table) = txn.open_table(TABLE) else {
                return false;
            };
            let packed = pack(&val, ttl);
            if table.insert(key, packed.as_slice()).is_err() {
                return false;
            }
        }
        txn.commit().is_ok()
    }

    fn remove_sync(&self, key: &str) -> bool {
        let Ok(txn) = self.db.begin_write() else {
            return false;
        };
        {
            let Ok(mut table) = txn.open_table(TABLE) else {
                return false;
            };
            let _ = table.remove(key);
        }
        txn.commit().is_ok()
    }

    fn incr_sync(&self, key: &str, by: i64, ttl: Option<Duration>) -> u64 {
        let Ok(txn) = self.db.begin_write() else {
            return 0;
        };
        let next = {
            let Ok(mut table) = txn.open_table(TABLE) else {
                return 0;
            };
            let cur = match table.get(key) {
                Ok(Some(g)) => {
                    let raw = g.value();
                    if is_expired(raw) {
                        0i64
                    } else {
                        unpack(raw)
                            .and_then(|b| std::str::from_utf8(&b).ok()?.parse().ok())
                            .unwrap_or(0)
                    }
                }
                _ => 0i64,
            };
            let next = (cur + by).max(0) as u64;
            let packed = pack(next.to_string().as_bytes(), ttl);
            if table.insert(key, packed.as_slice()).is_err() {
                return 0;
            }
            next
        };
        if txn.commit().is_err() {
            return 0;
        }
        next
    }

    fn clear_prefix_sync(&self, prefix: &str) -> u64 {
        let Ok(txn) = self.db.begin_write() else {
            return 0;
        };
        let total = {
            let Ok(mut table) = txn.open_table(TABLE) else {
                return 0;
            };
            let keys: Vec<String> = match table.range(prefix..) {
                Ok(range) => range
                    .filter_map(|r| r.ok())
                    .map(|(k, _)| k.value().to_string())
                    .take_while(|k| k.starts_with(prefix))
                    .collect(),
                Err(_) => return 0,
            };
            let n = keys.len() as u64;
            for k in keys {
                let _ = table.remove(k.as_str());
            }
            n
        };
        if txn.commit().is_err() {
            return 0;
        }
        total
    }
}

impl KvStore for RedbStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>> {
        let store = self.clone();
        let key = key.to_string();
        let key_log = trunc(&key);
        Box::pin(async move {
            let started = Instant::now();
            let val = tokio::task::spawn_blocking(move || store.get_sync(&key))
                .await
                .unwrap_or(None);
            let hit = val.is_some();
            let n = val.as_ref().map(|b| b.len() as u64);
            tracing::debug!(
                target: "sova.store",
                op = "get",
                backend = "redb",
                key = %key_log,
                hit,
                bytes = n,
                ok = true,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = rid().as_deref().unwrap_or(""),
                "sova.store"
            );
            val
        })
    }

    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()> {
        let store = self.clone();
        let key = key.to_string();
        let key_log = trunc(&key);
        Box::pin(async move {
            let started = Instant::now();
            let n = val.len() as u64;
            let ok = tokio::task::spawn_blocking(move || store.set_sync(&key, val, ttl))
                .await
                .unwrap_or(false);
            tracing::debug!(
                target: "sova.store",
                op = "set",
                backend = "redb",
                key = %key_log,
                bytes = n,
                ok,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = rid().as_deref().unwrap_or(""),
                "sova.store"
            );
        })
    }

    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()> {
        let store = self.clone();
        let key = key.to_string();
        let key_log = trunc(&key);
        Box::pin(async move {
            let started = Instant::now();
            let ok = tokio::task::spawn_blocking(move || store.remove_sync(&key))
                .await
                .unwrap_or(false);
            tracing::debug!(
                target: "sova.store",
                op = "remove",
                backend = "redb",
                key = %key_log,
                ok,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = rid().as_deref().unwrap_or(""),
                "sova.store"
            );
        })
    }

    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64> {
        let store = self.clone();
        let key = key.to_string();
        let key_log = trunc(&key);
        Box::pin(async move {
            let started = Instant::now();
            let next = tokio::task::spawn_blocking(move || store.incr_sync(&key, by, ttl))
                .await
                .unwrap_or(0);
            tracing::debug!(
                target: "sova.store",
                op = "incr",
                backend = "redb",
                key = %key_log,
                ok = true,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = rid().as_deref().unwrap_or(""),
                "sova.store"
            );
            next
        })
    }

    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64> {
        let store = self.clone();
        let prefix = prefix.to_string();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || store.clear_prefix_sync(&prefix))
                .await
                .unwrap_or(0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conformance;
    use std::sync::Arc;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn redb_conformance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kv.redb");
        let store = Arc::new(RedbStore::open(&path).unwrap());
        conformance::run(store).await;
    }

    #[tokio::test]
    async fn redb_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("persist.redb");
        {
            let store = RedbStore::open(&path).unwrap();
            store.set("k", Bytes::from_static(b"v"), None).await;
        }
        let store = RedbStore::open(&path).unwrap();
        assert_eq!(store.get("k").await.as_deref(), Some(b"v".as_slice()));
    }
}
