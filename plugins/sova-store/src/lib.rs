//! Byte-oriented key-value store for Sova plugins (sessions, cache, CSRF, rate-limit).
//!
//! Trait is stable (memory + file + sql + redis + redb backends).
//! Enable feature `unstable-store` for backwards-compatible feature flags.
//! **Not in sova-core** — wire with `app.state(store.namespace("sess"))`.

mod cache;
#[cfg(feature = "store-crypto")]
mod encrypted;
#[cfg(feature = "file")]
mod file;
#[cfg(feature = "redb")]
mod redb;
#[cfg(feature = "redis")]
mod redis;
#[cfg(feature = "sql")]
mod sql;

use bytes::Bytes;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub use cache::{Cache, CacheError};

pub trait KvStore: Send + Sync + 'static {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>>;
    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()>;
    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()>;
    /// Atomic increment; creates the key at `by` when missing. Returns new value.
    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64>;
    /// Remove keys starting with `prefix`. Returns how many were removed.
    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64>;
}

pub fn namespace(store: Arc<dyn KvStore>, name: &str) -> Namespace {
    Namespace {
        store,
        prefix: format!("{name}:"),
    }
}

/// App-wide store handle — `app.state(AppStore::memory())` or via facade `SharedStore` plugin.
/// Session / meta / rate-limit can reuse namespaced views of the same backend.
#[derive(Clone)]
pub struct AppStore {
    pub inner: Arc<dyn KvStore>,
}

impl AppStore {
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self { inner: store }
    }

    pub fn memory() -> Self {
        Self::new(Arc::new(MemoryStore::new()))
    }

    pub fn namespaced(&self, name: &str) -> Arc<dyn KvStore> {
        Arc::new(namespace(Arc::clone(&self.inner), name))
    }
}

#[cfg(feature = "store-crypto")]
pub use encrypted::{encrypted, encrypted_ns, AppKey, Encrypted};

#[cfg(feature = "file")]
pub use file::{Durability, FileStore};

#[cfg(feature = "redb")]
pub use redb::RedbStore;

#[cfg(feature = "redis")]
pub use redis::RedisStore;

#[cfg(feature = "sql")]
pub use sql::SqlStore;

/// Scoped handle — prefixes all keys; [`clear_prefix`](KvStore::clear_prefix) stays inside.
#[derive(Clone)]
pub struct Namespace {
    store: Arc<dyn KvStore>,
    prefix: String,
}

impl Namespace {
    fn full(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }
}

impl KvStore for Namespace {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>> {
        let k = self.full(key);
        Box::pin(async move { self.store.get(&k).await })
    }

    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()> {
        let k = self.full(key);
        Box::pin(async move { self.store.set(&k, val, ttl).await })
    }

    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()> {
        let k = self.full(key);
        Box::pin(async move { self.store.remove(&k).await })
    }

    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64> {
        let k = self.full(key);
        Box::pin(async move { self.store.incr(&k, by, ttl).await })
    }

    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64> {
        let p = self.full(prefix);
        Box::pin(async move { self.store.clear_prefix(&p).await })
    }
}

struct Entry {
    val: Bytes,
    exp: Option<Instant>,
}

type ShardMap = Arc<Mutex<HashMap<String, Entry>>>;

/// In-process KvStore (sharded Mutex maps + TTL).
#[derive(Clone)]
pub struct MemoryStore {
    shards: Arc<[ShardMap]>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    /// Default shard count: `max(1, available_parallelism * 2)`.
    pub fn new() -> Self {
        let n = std::thread::available_parallelism()
            .map(|p| p.get() * 2)
            .unwrap_or(2)
            .max(1);
        Self::with_shards(n)
    }

    /// Explicit shard count (minimum 1).
    pub fn with_shards(n: usize) -> Self {
        let n = n.max(1);
        let shards: Vec<_> = (0..n)
            .map(|_| Arc::new(Mutex::new(HashMap::new())))
            .collect();
        Self {
            shards: shards.into(),
        }
    }

    fn shard(&self, key: &str) -> &ShardMap {
        &self.shards[shard_index(key, self.shards.len())]
    }

    fn alive(e: &Entry, now: Instant) -> bool {
        e.exp.map(|t| t > now).unwrap_or(true)
    }
}

fn shard_index(key: &str, n: usize) -> usize {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    (h.finish() as usize) % n
}

fn trunc_key(key: &str) -> String {
    const MAX: usize = 120;
    if key.len() <= MAX {
        key.to_string()
    } else {
        format!("{}…", &key[..MAX - 1])
    }
}

impl KvStore for MemoryStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>> {
        let shard = Arc::clone(self.shard(key));
        let key = key.to_string();
        Box::pin(async move {
            let started = Instant::now();
            let mut map = shard.lock().await;
            let now = Instant::now();
            let val = match map.get(&key) {
                Some(e) if Self::alive(e, now) => Some(e.val.clone()),
                Some(_) => {
                    map.remove(&key);
                    None
                }
                None => None,
            };
            let hit = val.is_some();
            let n = val.as_ref().map(|b| b.len() as u64);
            tracing::debug!(
                target: "sova.store",
                op = "get",
                backend = "memory",
                key = %trunc_key(&key),
                hit,
                bytes = n,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = sova_core::current_request_id().as_deref().unwrap_or(""),
                "sova.store"
            );
            val
        })
    }

    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()> {
        let shard = Arc::clone(self.shard(key));
        let key = key.to_string();
        Box::pin(async move {
            let started = Instant::now();
            let n = val.len() as u64;
            let mut map = shard.lock().await;
            map.insert(
                key.clone(),
                Entry {
                    val,
                    exp: ttl.map(|d| Instant::now() + d),
                },
            );
            tracing::debug!(
                target: "sova.store",
                op = "set",
                backend = "memory",
                key = %trunc_key(&key),
                bytes = n,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = sova_core::current_request_id().as_deref().unwrap_or(""),
                "sova.store"
            );
        })
    }

    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()> {
        let shard = Arc::clone(self.shard(key));
        let key = key.to_string();
        Box::pin(async move {
            shard.lock().await.remove(&key);
        })
    }

    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64> {
        let shard = Arc::clone(self.shard(key));
        let key = key.to_string();
        Box::pin(async move {
            let mut map = shard.lock().await;
            let now = Instant::now();
            let cur = match map.get(&key) {
                Some(e) if Self::alive(e, now) => {
                    let s = std::str::from_utf8(&e.val).unwrap_or("0");
                    s.parse::<i64>().unwrap_or(0)
                }
                _ => 0,
            };
            let next = (cur + by).max(0) as u64;
            map.insert(
                key,
                Entry {
                    val: Bytes::from(next.to_string()),
                    exp: ttl.map(|d| now + d),
                },
            );
            next
        })
    }

    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64> {
        let prefix = prefix.to_string();
        let shards: Vec<_> = self.shards.iter().cloned().collect();
        Box::pin(async move {
            let mut total = 0u64;
            for shard in shards {
                let mut map = shard.lock().await;
                let keys: Vec<_> = map
                    .keys()
                    .filter(|k| k.starts_with(&prefix))
                    .cloned()
                    .collect();
                total += keys.len() as u64;
                for k in keys {
                    map.remove(&k);
                }
            }
            total
        })
    }
}

/// Shared conformance suite for any [`KvStore`].
pub mod conformance {
    use super::*;
    use std::sync::Arc;

    pub async fn run(store: Arc<dyn KvStore>) {
        get_set_ttl(store.clone()).await;
        namespace_isolation(store.clone()).await;
        incr_atomic(store.clone()).await;
        clear_prefix_scoped(store).await;
    }

    async fn get_set_ttl(store: Arc<dyn KvStore>) {
        store
            .set("a", Bytes::from_static(b"1"), Some(Duration::from_millis(50)))
            .await;
        assert_eq!(store.get("a").await.as_deref(), Some(b"1".as_slice()));
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(store.get("a").await.is_none());
    }

    async fn namespace_isolation(store: Arc<dyn KvStore>) {
        let a = namespace(store.clone(), "a");
        let b = namespace(store.clone(), "b");
        a.set("k", Bytes::from_static(b"A"), None).await;
        b.set("k", Bytes::from_static(b"B"), None).await;
        assert_eq!(a.get("k").await.as_deref(), Some(b"A".as_slice()));
        assert_eq!(b.get("k").await.as_deref(), Some(b"B".as_slice()));
        a.clear_prefix("").await;
        assert!(a.get("k").await.is_none());
        assert_eq!(b.get("k").await.as_deref(), Some(b"B".as_slice()));
    }

    async fn incr_atomic(store: Arc<dyn KvStore>) {
        store.remove("c").await;
        let mut handles = Vec::new();
        for _ in 0..50 {
            let s = store.clone();
            handles.push(tokio::spawn(async move {
                s.incr("c", 1, None).await;
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(store.get("c").await.unwrap().as_ref(), b"50");
    }

    async fn clear_prefix_scoped(store: Arc<dyn KvStore>) {
        store.set("p:1", Bytes::from_static(b"x"), None).await;
        store.set("p:2", Bytes::from_static(b"y"), None).await;
        store.set("q:1", Bytes::from_static(b"z"), None).await;
        assert_eq!(store.clear_prefix("p:").await, 2);
        assert!(store.get("p:1").await.is_none());
        assert_eq!(store.get("q:1").await.as_deref(), Some(b"z".as_slice()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn memory_conformance() {
        conformance::run(Arc::new(MemoryStore::new())).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn sharded_clear_prefix_across_shards() {
        let store = MemoryStore::with_shards(4);
        for i in 0..32 {
            store
                .set(
                    &format!("shard:{i}"),
                    Bytes::from_static(b"v"),
                    None,
                )
                .await;
        }
        store
            .set("other:1", Bytes::from_static(b"z"), None)
            .await;
        assert_eq!(store.clear_prefix("shard:").await, 32);
        assert!(store.get("shard:0").await.is_none());
        assert_eq!(
            store.get("other:1").await.as_deref(),
            Some(b"z".as_slice())
        );
    }

    #[tokio::test]
    async fn cache_remember_memory() {
        let store = AppStore::memory();
        let cache = store.cache();
        let v = cache
            .remember("k", None, || async { Ok::<_, CacheError>(42u32) })
            .await
            .unwrap();
        assert_eq!(v, 42);
        let hit = cache.get_json::<u32>("k").await;
        assert_eq!(hit, Some(42));
    }
}
