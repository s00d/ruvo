//! Session persistence: [`SessionStore`] + [`KvSessionStore`] (+ optional SQL).

use bytes::Bytes;
use sova_store::{BoxFuture, KvStore};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

/// Session data key storing the authenticated user id ([`super::Session::bind_user`]).
pub const SESSION_USER_KEY: &str = "_sova_uid";

/// Backend for cookie sessions (load/save/destroy + logout-by-user).
pub trait SessionStore: Send + Sync + 'static {
    fn load<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Option<HashMap<String, String>>>;
    fn save<'a>(
        &'a self,
        id: &'a str,
        data: &'a HashMap<String, String>,
        ttl: Duration,
    ) -> BoxFuture<'a, ()>;
    fn destroy<'a>(&'a self, id: &'a str) -> BoxFuture<'a, ()>;
    /// Remove all sessions for `user_id`. If `keep_sid` is set, leave that id.
    fn destroy_user<'a>(
        &'a self,
        user_id: &'a str,
        keep_sid: Option<&'a str>,
    ) -> BoxFuture<'a, u64>;
}

/// App-state handle so handlers can call [`SessionStore::destroy_user`].
#[derive(Clone)]
pub struct SessionStoreHandle(pub Arc<dyn SessionStore>);

impl SessionStoreHandle {
    pub fn inner(&self) -> &Arc<dyn SessionStore> {
        &self.0
    }
}

pub(crate) fn encode(data: &HashMap<String, String>) -> Bytes {
    let mut out = String::new();
    for (k, v) in data {
        out.push_str(k);
        out.push('\0');
        out.push_str(v);
        out.push('\n');
    }
    Bytes::from(out)
}

pub(crate) fn decode(bytes: &[u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in std::str::from_utf8(bytes).unwrap_or("").lines() {
        if let Some((k, v)) = line.split_once('\0') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

fn index_key(user_id: &str) -> String {
    format!("uids:{user_id}")
}

fn decode_set(raw: &[u8]) -> HashSet<String> {
    serde_json::from_slice(raw).unwrap_or_default()
}

fn encode_set(set: &HashSet<String>) -> Bytes {
    Bytes::from(serde_json::to_vec(set).unwrap_or_else(|_| b"[]".to_vec()))
}

/// [`SessionStore`] over any [`KvStore`] (memory / file / redis / `sova_kv`).
///
/// Maintains secondary index keys `uids:{user_id}` → JSON set of session ids.
pub struct KvSessionStore {
    kv: Arc<dyn KvStore>,
}

impl KvSessionStore {
    pub fn new(kv: Arc<dyn KvStore>) -> Self {
        Self { kv }
    }

    async fn track(&self, user_id: &str, sid: &str, ttl: Duration) {
        if user_id.is_empty() || sid.is_empty() {
            return;
        }
        let key = index_key(user_id);
        let mut set = self
            .kv
            .get(&key)
            .await
            .map(|b| decode_set(&b))
            .unwrap_or_default();
        set.insert(sid.to_string());
        self.kv.set(&key, encode_set(&set), Some(ttl)).await;
    }

    async fn untrack(&self, user_id: &str, sid: &str, ttl: Duration) {
        if user_id.is_empty() || sid.is_empty() {
            return;
        }
        let key = index_key(user_id);
        let Some(raw) = self.kv.get(&key).await else {
            return;
        };
        let mut set = decode_set(&raw);
        if set.remove(sid) {
            if set.is_empty() {
                self.kv.remove(&key).await;
            } else {
                self.kv.set(&key, encode_set(&set), Some(ttl)).await;
            }
        }
    }
}

impl SessionStore for KvSessionStore {
    fn load<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Option<HashMap<String, String>>> {
        Box::pin(async move { self.kv.get(id).await.map(|b| decode(&b)) })
    }

    fn save<'a>(
        &'a self,
        id: &'a str,
        data: &'a HashMap<String, String>,
        ttl: Duration,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Some(prev) = self.kv.get(id).await.map(|b| decode(&b)) {
                let old_uid = prev.get(SESSION_USER_KEY).map(String::as_str);
                let new_uid = data.get(SESSION_USER_KEY).map(String::as_str);
                if old_uid != new_uid {
                    if let Some(u) = old_uid {
                        self.untrack(u, id, ttl).await;
                    }
                }
            }
            self.kv.set(id, encode(data), Some(ttl)).await;
            if let Some(uid) = data.get(SESSION_USER_KEY).filter(|s| !s.is_empty()) {
                self.track(uid, id, ttl).await;
            }
        })
    }

    fn destroy<'a>(&'a self, id: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Some(data) = self.kv.get(id).await.map(|b| decode(&b)) {
                if let Some(uid) = data.get(SESSION_USER_KEY).filter(|s| !s.is_empty()) {
                    // TTL only refreshes index; use a day if unknown.
                    self.untrack(uid, id, Duration::from_secs(86400)).await;
                }
            }
            self.kv.remove(id).await;
        })
    }

    fn destroy_user<'a>(
        &'a self,
        user_id: &'a str,
        keep_sid: Option<&'a str>,
    ) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            if user_id.is_empty() {
                return 0;
            }
            let key = index_key(user_id);
            let Some(raw) = self.kv.get(&key).await else {
                return 0;
            };
            let set = decode_set(&raw);
            let mut kept = HashSet::new();
            let mut n = 0u64;
            for sid in set {
                if keep_sid.is_some_and(|k| k == sid) {
                    kept.insert(sid);
                    continue;
                }
                self.kv.remove(&sid).await;
                n += 1;
            }
            if kept.is_empty() {
                self.kv.remove(&key).await;
            } else {
                self.kv
                    .set(&key, encode_set(&kept), Some(Duration::from_secs(86400)))
                    .await;
            }
            n
        })
    }
}
