//! Redis-backed [`KvStore`] on the shared [`sova_redis::RedisPool`].

use crate::{BoxFuture, KvStore};
use bytes::Bytes;
use redis::AsyncCommands;
use sova_redis::RedisPool;
use std::time::{Duration, Instant};

/// Key-value store on Redis / Valkey.
#[derive(Clone)]
pub struct RedisStore {
    pool: RedisPool,
    key_prefix: String,
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

impl RedisStore {
    pub fn new(pool: RedisPool) -> Self {
        Self {
            pool,
            key_prefix: String::new(),
        }
    }

    /// Bind to Sova [`RedisPool`] (pool may connect later via plugin startup).
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
            let started = Instant::now();
            let mut conn = match self.pool.get() {
                Ok(c) => c,
                Err(_) => {
                    tracing::debug!(
                        target: "sova.store",
                        op = "get",
                        backend = "redis",
                        key = %trunc(key),
                        hit = false,
                        ok = false,
                        duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                        request_id = rid().as_deref().unwrap_or(""),
                        "sova.store"
                    );
                    return None;
                }
            };
            let k = self.full_key(key);
            let val: Option<Vec<u8>> = match conn.get(k).await {
                Ok(v) => v,
                Err(_) => {
                    tracing::debug!(
                        target: "sova.store",
                        op = "get",
                        backend = "redis",
                        key = %trunc(key),
                        hit = false,
                        ok = false,
                        duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                        request_id = rid().as_deref().unwrap_or(""),
                        "sova.store"
                    );
                    return None;
                }
            };
            let hit = val.is_some();
            let n = val.as_ref().map(|b| b.len() as u64);
            tracing::debug!(
                target: "sova.store",
                op = "get",
                backend = "redis",
                key = %trunc(key),
                hit,
                bytes = n,
                ok = true,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = rid().as_deref().unwrap_or(""),
                "sova.store"
            );
            val.map(Bytes::from)
        })
    }

    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let started = Instant::now();
            let n = val.len() as u64;
            let Ok(mut conn) = self.pool.get() else {
                tracing::debug!(
                    target: "sova.store",
                    op = "set",
                    backend = "redis",
                    key = %trunc(key),
                    bytes = n,
                    ok = false,
                    duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                    request_id = rid().as_deref().unwrap_or(""),
                    "sova.store"
                );
                return;
            };
            let k = self.full_key(key);
            let bytes = val.as_ref();
            let ok = match ttl {
                Some(d) => {
                    let secs = d.as_secs().max(1);
                    conn.set_ex::<_, _, ()>(k, bytes, secs).await.is_ok()
                }
                None => conn.set::<_, _, ()>(k, bytes).await.is_ok(),
            };
            tracing::debug!(
                target: "sova.store",
                op = "set",
                backend = "redis",
                key = %trunc(key),
                bytes = n,
                ok,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = rid().as_deref().unwrap_or(""),
                "sova.store"
            );
        })
    }

    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let started = Instant::now();
            let Ok(mut conn) = self.pool.get() else {
                tracing::debug!(
                    target: "sova.store",
                    op = "remove",
                    backend = "redis",
                    key = %trunc(key),
                    ok = false,
                    duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                    request_id = rid().as_deref().unwrap_or(""),
                    "sova.store"
                );
                return;
            };
            let k = self.full_key(key);
            let ok = conn.del::<_, ()>(k).await.is_ok();
            tracing::debug!(
                target: "sova.store",
                op = "remove",
                backend = "redis",
                key = %trunc(key),
                ok,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = rid().as_deref().unwrap_or(""),
                "sova.store"
            );
        })
    }

    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let started = Instant::now();
            let Ok(mut conn) = self.pool.get() else {
                tracing::debug!(
                    target: "sova.store",
                    op = "incr",
                    backend = "redis",
                    key = %trunc(key),
                    ok = false,
                    duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                    request_id = rid().as_deref().unwrap_or(""),
                    "sova.store"
                );
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
                Err(_) => {
                    tracing::debug!(
                        target: "sova.store",
                        op = "incr",
                        backend = "redis",
                        key = %trunc(key),
                        ok = false,
                        duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                        request_id = rid().as_deref().unwrap_or(""),
                        "sova.store"
                    );
                    return 0;
                }
            };
            if let Some(d) = ttl {
                let secs = d.as_secs().max(1);
                let _: Result<(), _> = conn.expire(&k, secs as i64).await;
            }
            tracing::debug!(
                target: "sova.store",
                op = "incr",
                backend = "redis",
                key = %trunc(key),
                ok = true,
                duration_ms = started.elapsed().as_secs_f64() * 1000.0,
                request_id = rid().as_deref().unwrap_or(""),
                "sova.store"
            );
            next.max(0) as u64
        })
    }

    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let Ok(mut conn) = self.pool.get() else {
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
