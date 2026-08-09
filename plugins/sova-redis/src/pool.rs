use crate::RedisError;
use redis::aio::ConnectionManager;
use redis::Client;
use sova_core::Request;
use std::sync::{Arc, RwLock};

#[derive(Default)]
struct PoolInner {
    conn: Option<ConnectionManager>,
    url: Option<String>,
}

/// Shared Redis handle filled during `on_startup` (mirrors [`sova_db::DbPool`]).
#[derive(Clone, Default)]
pub struct RedisPool {
    inner: Arc<RwLock<PoolInner>>,
}

impl RedisPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remember `REDIS_URL` so Pub/Sub can open a dedicated connection.
    pub fn set_url(&self, url: impl Into<String>) {
        self.inner.write().unwrap().url = Some(url.into());
    }

    pub fn url(&self) -> Result<String, RedisError> {
        self.inner
            .read()
            .unwrap()
            .url
            .clone()
            .ok_or_else(|| RedisError::msg("redis url not set"))
    }

    pub fn set(&self, conn: ConnectionManager) {
        self.inner.write().unwrap().conn = Some(conn);
    }

    pub fn get(&self) -> Result<ConnectionManager, RedisError> {
        self.inner
            .read()
            .unwrap()
            .conn
            .clone()
            .ok_or_else(|| RedisError::msg("redis not connected"))
    }

    pub fn clear(&self) {
        let mut g = self.inner.write().unwrap();
        g.conn.take();
    }

    /// Connect and ping; used by the plugin and tests.
    pub async fn connect(url: &str) -> Result<ConnectionManager, RedisError> {
        let client = Client::open(url).map_err(RedisError::from)?;
        let mut conn = ConnectionManager::new(client)
            .await
            .map_err(RedisError::from)?;
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(RedisError::from)?;
        Ok(conn)
    }
}

/// Convenient `req.redis()`.
pub trait RedisExt {
    fn redis(&self) -> &RedisPool;
}

impl RedisExt for Request {
    fn redis(&self) -> &RedisPool {
        self.get::<RedisPool>()
            .expect("Redis plugin is not installed (missing req.redis())")
    }
}
