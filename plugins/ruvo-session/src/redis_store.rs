//! Redis-backed [`SessionStore`] (payload keys + SET index per user).

use super::store::{decode, encode, SessionStore, SESSION_USER_KEY};
use redis::AsyncCommands;
use ruvo_redis::RedisPool;
use ruvo_store::BoxFuture;
use std::collections::HashMap;
use std::time::Duration;

/// Sessions on Redis / Valkey: `{prefix}{id}` payload + `{prefix}uids:{user}` SET of sids.
#[derive(Clone)]
pub struct RedisSessionStore {
    pool: RedisPool,
    /// Key prefix (default `sess:`).
    prefix: String,
}

impl RedisSessionStore {
    pub fn new(pool: RedisPool) -> Self {
        Self {
            pool,
            prefix: "sess:".into(),
        }
    }

    pub fn from_redis_pool(pool: &RedisPool) -> Self {
        Self::new(pool.clone())
    }

    pub fn with_prefix(pool: &RedisPool, prefix: impl Into<String>) -> Self {
        Self {
            pool: pool.clone(),
            prefix: prefix.into(),
        }
    }

    fn payload_key(&self, id: &str) -> String {
        format!("{}{id}", self.prefix)
    }

    fn index_key(&self, user_id: &str) -> String {
        format!("{}uids:{user_id}", self.prefix)
    }

    async fn track(&self, user_id: &str, sid: &str, ttl: Duration) {
        if user_id.is_empty() || sid.is_empty() {
            return;
        }
        let Ok(mut conn) = self.pool.get().await else {
            return;
        };
        let key = self.index_key(user_id);
        let _: Result<(), _> = conn.sadd(&key, sid).await;
        let secs = ttl.as_secs().max(1) as i64;
        let _: Result<(), _> = conn.expire(&key, secs).await;
    }

    async fn untrack(&self, user_id: &str, sid: &str) {
        if user_id.is_empty() || sid.is_empty() {
            return;
        }
        let Ok(mut conn) = self.pool.get().await else {
            return;
        };
        let key = self.index_key(user_id);
        let _: Result<(), _> = conn.srem(&key, sid).await;
    }
}

impl SessionStore for RedisSessionStore {
    fn load<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Option<HashMap<String, String>>> {
        Box::pin(async move {
            let mut conn = match self.pool.get().await {
                Ok(c) => c,
                Err(_) => return None,
            };
            let k = self.payload_key(id);
            let val: Option<Vec<u8>> = match conn.get(k).await {
                Ok(v) => v,
                Err(_) => return None,
            };
            val.map(|b| decode(&b))
        })
    }

    fn save<'a>(
        &'a self,
        id: &'a str,
        data: &'a HashMap<String, String>,
        ttl: Duration,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Some(prev) = self.load(id).await {
                let old_uid = prev.get(SESSION_USER_KEY).map(String::as_str);
                let new_uid = data.get(SESSION_USER_KEY).map(String::as_str);
                if old_uid != new_uid {
                    if let Some(u) = old_uid {
                        self.untrack(u, id).await;
                    }
                }
            }

            let Ok(mut conn) = self.pool.get().await else {
                return;
            };
            let k = self.payload_key(id);
            let payload = encode(data);
            let secs = ttl.as_secs().max(1);
            let _: Result<(), _> = conn.set_ex(k, payload.as_ref(), secs).await;

            if let Some(uid) = data.get(SESSION_USER_KEY).filter(|s| !s.is_empty()) {
                self.track(uid, id, ttl).await;
            }
        })
    }

    fn destroy<'a>(&'a self, id: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let uid = self
                .load(id)
                .await
                .and_then(|d| d.get(SESSION_USER_KEY).filter(|s| !s.is_empty()).cloned());
            let Ok(mut conn) = self.pool.get().await else {
                return;
            };
            let k = self.payload_key(id);
            let _: Result<(), _> = conn.del(k).await;
            if let Some(uid) = uid {
                self.untrack(&uid, id).await;
            }
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
            let Ok(mut conn) = self.pool.get().await else {
                return 0;
            };
            let idx = self.index_key(user_id);
            let members: Vec<String> = match conn.smembers(&idx).await {
                Ok(m) => m,
                Err(_) => return 0,
            };
            let mut n = 0u64;
            let mut keep = false;
            for sid in members {
                if keep_sid.is_some_and(|k| k == sid) {
                    keep = true;
                    continue;
                }
                let pk = self.payload_key(&sid);
                let _: Result<(), _> = conn.del(&pk).await;
                let _: Result<(), _> = conn.srem(&idx, &sid).await;
                n += 1;
            }
            if !keep {
                let _: Result<(), _> = conn.del(&idx).await;
            }
            n
        })
    }
}
