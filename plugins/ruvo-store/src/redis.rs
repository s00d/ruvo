//! Redis-backed [`KvStore`] on the shared [`ruvo_redis::RedisPool`].

use crate::{BoxFuture, KvStore};
use bytes::Bytes;
use redis::AsyncCommands;
use ruvo_redis::RedisPool;
use std::time::Duration;

/// Key-value store on Redis / Valkey.
#[derive(Clone)]
pub struct RedisStore {
    pool: RedisPool,
    key_prefix: String,
}

impl RedisStore {
    pub fn new(pool: RedisPool) -> Self {
        Self {
            pool,
            key_prefix: String::new(),
        }
    }

    /// Bind to Ruvo [`RedisPool`] (pool may connect later via plugin startup).
    pub fn from_redis_pool(pool: &RedisPool) -> Self {
        Self::new(pool.clone())
    }

    /// Optional global key prefix (before namespace prefixes from [`crate::Namespace`]).
    pub fn with_prefix(pool: &RedisPool, prefix: impl Into<String>) -> Self {
        Self {
            pool: pool.clone(),
            key_prefix: prefix.into(),
        }
    }

    fn full_key(&self, key: &str) -> String {
        if self.key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}{key}", self.key_prefix)
        }
    }
}

impl KvStore for RedisStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>> {
        Box::pin(async move {
            let mut conn = match self.pool.get().await {
                Ok(c) => c,
                Err(_) => return None,
            };
            let k = self.full_key(key);
            let val: Option<Vec<u8>> = match conn.get(k).await {
                Ok(v) => v,
                Err(_) => return None,
            };
            val.map(Bytes::from)
        })
    }

    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let Ok(mut conn) = self.pool.get().await else {
                return;
            };
            let k = self.full_key(key);
            let bytes = val.as_ref();
            let _: Result<(), _> = match ttl {
                Some(d) => {
                    let secs = d.as_secs().max(1);
                    conn.set_ex(k, bytes, secs).await
                }
                None => conn.set(k, bytes).await,
            };
        })
    }

    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let Ok(mut conn) = self.pool.get().await else {
                return;
            };
            let k = self.full_key(key);
            let _: Result<(), _> = conn.del(k).await;
        })
    }

    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let Ok(mut conn) = self.pool.get().await else {
                return 0;
            };
            let k = self.full_key(key);
            let next: i64 = match redis::cmd("INCRBY")
                .arg(&k)
                .arg(by)
                .query_async(&mut conn)
                .await
            {
                Ok(n) => n,
                Err(_) => return 0,
            };
            if let Some(d) = ttl {
                let secs = d.as_secs().max(1);
                let _: Result<(), _> = conn.expire(&k, secs as i64).await;
            }
            next.max(0) as u64
        })
    }

    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let Ok(mut conn) = self.pool.get().await else {
                return 0;
            };
            let pattern = format!("{}{}*", self.key_prefix, prefix);
            let mut cursor: u64 = 0;
            let mut total = 0u64;
            loop {
                let (next, keys): (u64, Vec<String>) = match redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(&pattern)
                    .arg("COUNT")
                    .arg(100u64)
                    .query_async(&mut conn)
                    .await
                {
                    Ok(v) => v,
                    Err(_) => break,
                };
                if !keys.is_empty() {
                    let n: u64 = match conn.del(keys).await {
                        Ok(n) => n,
                        Err(_) => break,
                    };
                    total += n;
                }
                cursor = next;
                if cursor == 0 {
                    break;
                }
            }
            total
        })
    }
}
