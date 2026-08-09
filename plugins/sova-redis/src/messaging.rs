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

fn rid() -> Option<String> {
    sova_core::current_request_id()
}

fn trunc(s: &str) -> String {
    const MAX: usize = 120;
    if s.len() <= MAX {
        s.to_string()
    } else {
        format!("{}…", &s[..MAX - 1])
    }
}

impl RedisPool {
    /// `PUBLISH channel message` — returns subscriber count that received it.
    pub async fn publish(
        &self,
        channel: impl AsRef<str>,
        message: impl AsRef<[u8]>,
    ) -> Result<i64, RedisError> {
        let started = std::time::Instant::now();
        let ch = channel.as_ref();
        let mut conn = self.get()?;
        let res = conn
            .publish(ch, message.as_ref())
            .await
            .map_err(RedisError::from);
        let ok = res.is_ok();
        tracing::debug!(
            target: "sova.redis",
            cmd = "publish",
            key = %trunc(ch),
            channel = %trunc(ch),
            ok,
            duration_ms = started.elapsed().as_secs_f64() * 1000.0,
            request_id = rid().as_deref().unwrap_or(""),
            "sova.redis"
        );
        res
    }

    /// Open a dedicated Pub/Sub connection and subscribe to `channels`.
    ///
    /// Requires the pool URL (set by the [`crate::Redis`] plugin). Do not reuse
    /// the shared command connection for subscribe.
    pub async fn subscribe(
        &self,
        channels: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<RedisSubscriber, RedisError> {
        let url = self.url()?;
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
        let url = self.url()?;
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
        let started = std::time::Instant::now();
        let q = queue.as_ref();
        let mut conn = self.get()?;
        let res = conn
            .lpush(q, message.as_ref())
            .await
            .map_err(RedisError::from);
        let ok = res.is_ok();
        tracing::debug!(
            target: "sova.redis",
            cmd = "enqueue",
            key = %trunc(q),
            queue = %trunc(q),
            ok,
            duration_ms = started.elapsed().as_secs_f64() * 1000.0,
            request_id = rid().as_deref().unwrap_or(""),
            "sova.redis"
        );
        res
    }

    /// Non-blocking pop (`RPOP`). `None` if empty.
    pub async fn dequeue(&self, queue: impl AsRef<str>) -> Result<Option<Vec<u8>>, RedisError> {
        let started = std::time::Instant::now();
        let q = queue.as_ref();
        let mut conn = self.get()?;
        let res: Result<Option<Vec<u8>>, RedisError> =
            conn.rpop(q, None).await.map_err(RedisError::from);
        let ok = res.is_ok();
        let hit = matches!(res, Ok(Some(_)));
        tracing::debug!(
            target: "sova.redis",
            cmd = "dequeue",
            key = %trunc(q),
            queue = %trunc(q),
            hit,
            ok,
            duration_ms = started.elapsed().as_secs_f64() * 1000.0,
            request_id = rid().as_deref().unwrap_or(""),
            "sova.redis"
        );
        res
    }

    /// Blocking pop (`BRPOP`). `timeout = 0` waits forever.
    ///
    /// Returns `(queue_name, payload)` or `None` on timeout.
    pub async fn dequeue_wait(
        &self,
        queue: impl AsRef<str>,
        timeout: Duration,
    ) -> Result<Option<(String, Vec<u8>)>, RedisError> {
        let started = std::time::Instant::now();
        let q = queue.as_ref();
        let mut conn = self.get()?;
        let secs = timeout.as_secs() as f64 + f64::from(timeout.subsec_nanos()) / 1e9;
        let res: Result<Option<(String, Vec<u8>)>, RedisError> = redis::cmd("BRPOP")
            .arg(q)
            .arg(secs)
            .query_async(&mut conn)
            .await
            .map_err(RedisError::from);
        let ok = res.is_ok();
        let hit = matches!(res, Ok(Some(_)));
        tracing::debug!(
            target: "sova.redis",
            cmd = "dequeue_wait",
            key = %trunc(q),
            queue = %trunc(q),
            hit,
            ok,
            duration_ms = started.elapsed().as_secs_f64() * 1000.0,
            request_id = rid().as_deref().unwrap_or(""),
            "sova.redis"
        );
        res
    }
}
