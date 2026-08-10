//! In-memory Redis for tests and demos (`Redis::fake()`).

use crate::error::RedisError;
use crate::messaging::RedisMessage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

const MAX_DB: u8 = 15;

#[derive(Clone)]
struct Entry {
    value: String,
    expires: Option<Instant>,
}

#[derive(Default)]
struct Db {
    keys: HashMap<String, Entry>,
}

struct Inner {
    dbs: [Db; 16],
    channels: HashMap<String, broadcast::Sender<Vec<u8>>>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            dbs: std::array::from_fn(|_| Db::default()),
            channels: HashMap::new(),
        }
    }
}

/// Shared in-memory Redis (16 DBs, strings, pub/sub, SCAN).
#[derive(Clone, Default)]
pub struct FakeRedis {
    inner: Arc<Mutex<Inner>>,
}

impl FakeRedis {
    pub fn new() -> Self {
        Self::default()
    }

    fn purge_expired(db: &mut Db) {
        let now = Instant::now();
        db.keys.retain(|_, e| e.expires.is_none_or(|t| t > now));
    }

    fn db(inner: &mut Inner, db: u8) -> &mut Db {
        &mut inner.dbs[db.min(MAX_DB) as usize]
    }

    pub fn select(&self, db: u8) -> Result<(), RedisError> {
        let _ = db.min(MAX_DB);
        Ok(())
    }

    pub fn get(&self, db: u8, key: &str) -> Result<Option<String>, RedisError> {
        let mut g = self.inner.lock().unwrap();
        let db = Self::db(&mut g, db);
        Self::purge_expired(db);
        Ok(db.keys.get(key).map(|e| e.value.clone()))
    }

    pub fn set(&self, db: u8, key: &str, value: &str) -> Result<(), RedisError> {
        let mut g = self.inner.lock().unwrap();
        let db = Self::db(&mut g, db);
        db.keys.insert(
            key.to_string(),
            Entry {
                value: value.to_string(),
                expires: None,
            },
        );
        Ok(())
    }

    pub fn setex(&self, db: u8, key: &str, ttl_secs: u64, value: &str) -> Result<(), RedisError> {
        let mut g = self.inner.lock().unwrap();
        let db = Self::db(&mut g, db);
        db.keys.insert(
            key.to_string(),
            Entry {
                value: value.to_string(),
                expires: Some(Instant::now() + Duration::from_secs(ttl_secs)),
            },
        );
        Ok(())
    }

    pub fn del(&self, db: u8, key: &str) -> Result<i64, RedisError> {
        let mut g = self.inner.lock().unwrap();
        let db = Self::db(&mut g, db);
        Ok(if db.keys.remove(key).is_some() { 1 } else { 0 })
    }

    pub fn ttl(&self, db: u8, key: &str) -> Result<i64, RedisError> {
        let mut g = self.inner.lock().unwrap();
        let db = Self::db(&mut g, db);
        Self::purge_expired(db);
        match db.keys.get(key) {
            None => Ok(-2),
            Some(e) => match e.expires {
                None => Ok(-1),
                Some(t) => {
                    let left = t.saturating_duration_since(Instant::now());
                    Ok(left.as_secs() as i64)
                }
            },
        }
    }

    pub fn key_type(&self, db: u8, key: &str) -> Result<String, RedisError> {
        let mut g = self.inner.lock().unwrap();
        let db = Self::db(&mut g, db);
        Self::purge_expired(db);
        Ok(if db.keys.contains_key(key) {
            "string".into()
        } else {
            "none".into()
        })
    }

    pub fn scan(
        &self,
        db: u8,
        cursor: u64,
        pattern: &str,
        count: usize,
    ) -> Result<(u64, Vec<String>), RedisError> {
        let mut g = self.inner.lock().unwrap();
        let db = Self::db(&mut g, db);
        Self::purge_expired(db);
        let mut keys: Vec<String> = db
            .keys
            .keys()
            .filter(|k| glob_match(pattern, k))
            .cloned()
            .collect();
        keys.sort();
        let start = cursor as usize;
        let end = (start + count).min(keys.len());
        let slice = keys[start..end].to_vec();
        let next = if end >= keys.len() { 0 } else { end as u64 };
        Ok((next, slice))
    }

    pub fn publish(&self, channel: &str, message: &[u8]) -> Result<i64, RedisError> {
        let mut g = self.inner.lock().unwrap();
        let tx = g
            .channels
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(64).0);
        Ok(tx.send(message.to_vec()).unwrap_or(0) as i64)
    }

    pub fn subscribe(&self, channel: &str) -> FakeSubscriber {
        let mut g = self.inner.lock().unwrap();
        let tx = g
            .channels
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(64).0);
        FakeSubscriber {
            rx: tx.subscribe(),
            channel: channel.to_string(),
        }
    }
}

pub struct FakeSubscriber {
    rx: broadcast::Receiver<Vec<u8>>,
    channel: String,
}

impl FakeSubscriber {
    pub async fn next(&mut self) -> Option<RedisMessage> {
        loop {
            match self.rx.recv().await {
                Ok(payload) => {
                    return Some(RedisMessage {
                        channel: self.channel.clone(),
                        payload,
                        pattern: None,
                    });
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }
}

fn glob_match(pattern: &str, key: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return key.starts_with(prefix);
    }
    pattern == key
}
