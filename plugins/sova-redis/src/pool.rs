use crate::error::RedisError;
use crate::fake::FakeRedis;
use crate::fake::FakeSubscriber;
use redis::aio::ConnectionManager;
use redis::Client;
use sova_core::Request;
use std::sync::{Arc, RwLock};

#[derive(Default)]
struct PoolInner {
    conn: Option<ConnectionManager>,
    url: Option<String>,
    fake: Option<Arc<FakeRedis>>,
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

    pub fn is_fake(&self) -> bool {
        self.inner.read().unwrap().fake.is_some()
    }

    pub fn fake(&self) -> Option<Arc<FakeRedis>> {
        self.inner.read().unwrap().fake.clone()
    }

    pub fn set_fake(&self, fake: FakeRedis) {
        let mut g = self.inner.write().unwrap();
        g.fake = Some(Arc::new(fake));
        g.url = Some("fake://memory".into());
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
        let g = self.inner.read().unwrap();
        if g.fake.is_some() {
            return Err(RedisError::msg("fake redis: use FakeRedis API"));
        }
        g.conn
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

    pub async fn subscribe_fake(
        &self,
        channel: impl AsRef<str>,
    ) -> Result<FakeSubscriber, RedisError> {
        let fake = self
            .fake()
            .ok_or_else(|| RedisError::msg("fake redis not configured"))?;
        Ok(fake.subscribe(channel.as_ref()))
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
