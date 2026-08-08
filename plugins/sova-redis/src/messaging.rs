//! Redis Pub/Sub + list queues on [`RedisPool`].

use crate::error::RedisError;
use crate::pool::RedisPool;
use futures_util::StreamExt;
use redis::AsyncCommands;
use std::time::Duration;

/// One Pub/Sub delivery.
#[derive(Debug, Clone)]
pub struct RedisMessage {
    pub channel: String,
    pub payload: Vec<u8>,
    /// Set when received via pattern subscribe (`PSUBSCRIBE`).
    pub pattern: Option<String>,
}

impl RedisMessage {
    pub fn payload_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.payload).ok()
    }
}

/// Dedicated Pub/Sub connection (not shared with normal commands).
pub struct RedisSubscriber {
    pubsub: redis::aio::PubSub,
}

impl RedisSubscriber {
    /// Wait for the next message (`None` when the connection closes).
    pub async fn next(&mut self) -> Option<RedisMessage> {
        let msg = self.pubsub.on_message().next().await?;
        let pattern = msg
            .get_pattern::<Option<String>>()
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        Some(RedisMessage {
            channel: msg.get_channel_name().to_string(),
            payload: msg.get_payload_bytes().to_vec(),
            pattern,
        })
    }

    pub async fn subscribe(&mut self, channel: impl AsRef<str>) -> Result<(), RedisError> {
        self.pubsub
            .subscribe(channel.as_ref())
            .await
            .map_err(RedisError::from)
    }

    pub async fn unsubscribe(&mut self, channel: impl AsRef<str>) -> Result<(), RedisError> {
        self.pubsub
            .unsubscribe(channel.as_ref())
            .await
            .map_err(RedisError::from)
    }

    pub async fn psubscribe(&mut self, pattern: impl AsRef<str>) -> Result<(), RedisError> {
        self.pubsub
            .psubscribe(pattern.as_ref())
            .await
            .map_err(RedisError::from)
    }

    pub async fn punsubscribe(&mut self, pattern: impl AsRef<str>) -> Result<(), RedisError> {
        self.pubsub
            .punsubscribe(pattern.as_ref())
            .await
            .map_err(RedisError::from)
    }
}

impl RedisPool {
    /// `PUBLISH channel message` — returns subscriber count that received it.
    pub async fn publish(
        &self,
        channel: impl AsRef<str>,
        message: impl AsRef<[u8]>,
    ) -> Result<i64, RedisError> {
        let mut conn = self.get().await?;
        let n: i64 = conn
            .publish(channel.as_ref(), message.as_ref())
            .await
            .map_err(RedisError::from)?;
        Ok(n)
    }

    /// Open a dedicated Pub/Sub connection and subscribe to `channels`.
    ///
    /// Requires the pool URL (set by the [`crate::Redis`] plugin). Do not reuse
    /// the shared command connection for subscribe.
    pub async fn subscribe(
        &self,
        channels: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<RedisSubscriber, RedisError> {
        let url = self.url().await?;
        let client = redis::Client::open(url.as_str()).map_err(RedisError::from)?;
        let mut pubsub = client.get_async_pubsub().await.map_err(RedisError::from)?;
        let names: Vec<String> = channels
            .into_iter()
            .map(|c| c.as_ref().to_string())
            .collect();
        if names.is_empty() {
            return Err(RedisError::msg("subscribe: no channels"));
        }
        pubsub.subscribe(&names).await.map_err(RedisError::from)?;
        Ok(RedisSubscriber { pubsub })
    }

    /// Pattern subscribe (`PSUBSCRIBE`), e.g. `events:*`.
    pub async fn psubscribe(
        &self,
        patterns: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<RedisSubscriber, RedisError> {
        let url = self.url().await?;
        let client = redis::Client::open(url.as_str()).map_err(RedisError::from)?;
        let mut pubsub = client.get_async_pubsub().await.map_err(RedisError::from)?;
        let names: Vec<String> = patterns
            .into_iter()
            .map(|c| c.as_ref().to_string())
            .collect();
        if names.is_empty() {
            return Err(RedisError::msg("psubscribe: no patterns"));
        }
        pubsub.psubscribe(&names).await.map_err(RedisError::from)?;
        Ok(RedisSubscriber { pubsub })
    }

    /// Push onto a list queue (`LPUSH`). Returns new list length.
    pub async fn enqueue(
        &self,
        queue: impl AsRef<str>,
        message: impl AsRef<[u8]>,
    ) -> Result<i64, RedisError> {
        let mut conn = self.get().await?;
        let n: i64 = conn
            .lpush(queue.as_ref(), message.as_ref())
            .await
            .map_err(RedisError::from)?;
        Ok(n)
    }

    /// Non-blocking pop (`RPOP`). `None` if empty.
    pub async fn dequeue(&self, queue: impl AsRef<str>) -> Result<Option<Vec<u8>>, RedisError> {
        let mut conn = self.get().await?;
        let val: Option<Vec<u8>> = conn.rpop(queue.as_ref(), None).await.map_err(RedisError::from)?;
        Ok(val)
    }

    /// Blocking pop (`BRPOP`). `timeout = 0` waits forever.
    ///
    /// Returns `(queue_name, payload)` or `None` on timeout.
    pub async fn dequeue_wait(
        &self,
        queue: impl AsRef<str>,
        timeout: Duration,
    ) -> Result<Option<(String, Vec<u8>)>, RedisError> {
        let mut conn = self.get().await?;
        let secs = timeout.as_secs() as f64 + f64::from(timeout.subsec_nanos()) / 1e9;
        let val: Option<(String, Vec<u8>)> = redis::cmd("BRPOP")
            .arg(queue.as_ref())
            .arg(secs)
            .query_async(&mut conn)
            .await
            .map_err(RedisError::from)?;
        Ok(val)
    }
}
