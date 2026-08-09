//! Task queue store for Sova.
//!
//! Trait is stable (memory + file + sql + redis backends).
//! Queue claim/lease is **not** plain KvStore.

#[cfg(feature = "file")]
mod file;
#[cfg(feature = "redis")]
mod redis;
#[cfg(feature = "sql")]
mod sql;

use bytes::Bytes;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[cfg(feature = "file")]
pub use file::FileTaskStore;

#[cfg(feature = "redis")]
pub use redis::RedisTaskStore;

#[cfg(feature = "sql")]
pub use sql::SqlTaskStore;

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("{0}")]
    Msg(String),
    #[error("not found")]
    NotFound,
    #[error("duplicate dedup key")]
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub queue: String,
    pub payload: Bytes,
    pub run_at: SystemTime,
    pub attempts: u32,
    pub lease_until: Option<SystemTime>,
    pub dedup_key: Option<String>,
    pub status: TaskStatus,
    pub worker: Option<String>,
    /// Higher values are claimed first within a queue (default `0`).
    pub priority: i32,
}

pub struct EnqueueOpts {
    pub queue: String,
    pub payload: Bytes,
    pub run_at: Option<SystemTime>,
    pub dedup_key: Option<String>,
    /// Higher values are claimed first (default `0`).
    pub priority: i32,
}

pub trait TaskStore: Send + Sync + 'static {
    fn enqueue<'a>(&'a self, opts: EnqueueOpts) -> BoxFuture<'a, Result<String, TaskError>>;
    fn claim<'a>(
        &'a self,
        queue: &'a str,
        worker: &'a str,
        lease: Duration,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Task>, TaskError>>;
    fn heartbeat<'a>(
        &'a self,
        id: &'a str,
        lease: Duration,
    ) -> BoxFuture<'a, Result<(), TaskError>>;
    fn complete<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<(), TaskError>>;
    fn fail<'a>(
        &'a self,
        id: &'a str,
        retry_at: Option<SystemTime>,
    ) -> BoxFuture<'a, Result<(), TaskError>>;
    fn reap<'a>(&'a self, now: SystemTime) -> BoxFuture<'a, Result<u64, TaskError>>;
    fn list<'a>(
        &'a self,
        queue: &'a str,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Task>, TaskError>>;
}

#[derive(Default)]
struct Inner {
    tasks: HashMap<String, Task>,
    dedup: HashMap<String, String>,
    seq: u64,
}

#[derive(Clone, Default)]
pub struct MemoryStore {
    inner: Arc<Mutex<Inner>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TaskStore for MemoryStore {
    fn enqueue<'a>(&'a self, opts: EnqueueOpts) -> BoxFuture<'a, Result<String, TaskError>> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            if let Some(ref dk) = opts.dedup_key {
                if let Some(existing) = g.dedup.get(dk) {
                    return Ok(existing.clone());
                }
            }
            g.seq += 1;
            let id = format!("t{}", g.seq);
            let task = Task {
                id: id.clone(),
                queue: opts.queue,
                payload: opts.payload,
                run_at: opts.run_at.unwrap_or_else(SystemTime::now),
                attempts: 0,
                lease_until: None,
                dedup_key: opts.dedup_key.clone(),
                status: TaskStatus::Pending,
                worker: None,
                priority: opts.priority,
            };
            if let Some(dk) = opts.dedup_key {
                g.dedup.insert(dk, id.clone());
            }
            g.tasks.insert(id.clone(), task);
            Ok(id)
        })
    }

    fn claim<'a>(
        &'a self,
        queue: &'a str,
        worker: &'a str,
        lease: Duration,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Task>, TaskError>> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            let now = SystemTime::now();
            let mut ids: Vec<_> = g
                .tasks
                .values()
                .filter(|t| {
                    t.queue == queue
                        && t.status == TaskStatus::Pending
                        && t.run_at <= now
                })
                .map(|t| (std::cmp::Reverse(t.priority), t.run_at, t.id.clone()))
                .collect();
            ids.sort();
            let mut out = Vec::new();
            for (_, _, id) in ids.into_iter().take(limit) {
                if let Some(t) = g.tasks.get_mut(&id) {
                    t.status = TaskStatus::Running;
                    t.attempts += 1;
                    t.worker = Some(worker.to_string());
                    t.lease_until = Some(now + lease);
                    out.push(t.clone());
                }
            }
            Ok(out)
        })
    }

    fn heartbeat<'a>(
        &'a self,
        id: &'a str,
        lease: Duration,
    ) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            let t = g.tasks.get_mut(id).ok_or(TaskError::NotFound)?;
            if t.status != TaskStatus::Running {
                return Err(TaskError::Msg("not running".into()));
            }
            t.lease_until = Some(SystemTime::now() + lease);
            Ok(())
        })
    }

    fn complete<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            let t = g.tasks.get_mut(id).ok_or(TaskError::NotFound)?;
            t.status = TaskStatus::Done;
            t.lease_until = None;
            t.worker = None;
            if let Some(dk) = t.dedup_key.clone() {
                g.dedup.remove(&dk);
            }
            Ok(())
        })
    }

    fn fail<'a>(
        &'a self,
        id: &'a str,
        retry_at: Option<SystemTime>,
    ) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            let t = g.tasks.get_mut(id).ok_or(TaskError::NotFound)?;
            if let Some(at) = retry_at {
                t.status = TaskStatus::Pending;
                t.run_at = at;
                t.lease_until = None;
                t.worker = None;
            } else {
                t.status = TaskStatus::Failed;
                t.lease_until = None;
                t.worker = None;
                if let Some(dk) = t.dedup_key.clone() {
                    g.dedup.remove(&dk);
                }
            }
            Ok(())
        })
    }

    fn reap<'a>(&'a self, now: SystemTime) -> BoxFuture<'a, Result<u64, TaskError>> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            let mut n = 0u64;
            for t in g.tasks.values_mut() {
                if t.status == TaskStatus::Running {
                    if let Some(until) = t.lease_until {
                        if until <= now {
                            t.status = TaskStatus::Pending;
                            t.lease_until = None;
                            t.worker = None;
                            n += 1;
                        }
                    }
                }
            }
            Ok(n)
        })
    }

    fn list<'a>(
        &'a self,
        queue: &'a str,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Task>, TaskError>> {
        Box::pin(async move {
            let g = self.inner.lock().await;
            let mut v: Vec<_> = g
                .tasks
                .values()
                .filter(|t| t.queue == queue)
                .cloned()
                .collect();
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v.truncate(limit);
            Ok(v)
        })
    }
}

pub mod conformance {
    use super::*;

    pub async fn run(store: Arc<dyn TaskStore>) {
        let id = store
            .enqueue(EnqueueOpts {
                queue: "default".into(),
                payload: Bytes::from_static(b"hi"),
                run_at: None,
                dedup_key: Some("once".into()),
                priority: 0,
            })
            .await
            .unwrap();
        let id2 = store
            .enqueue(EnqueueOpts {
                queue: "default".into(),
                payload: Bytes::from_static(b"hi2"),
                run_at: None,
                dedup_key: Some("once".into()),
                priority: 0,
            })
            .await
            .unwrap();
        assert_eq!(id, id2);

        let claimed = store
            .claim("default", "w1", Duration::from_secs(30), 10)
            .await
            .unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].id, id);

        store.heartbeat(&id, Duration::from_secs(30)).await.unwrap();
        store.complete(&id).await.unwrap();

        let id3 = store
            .enqueue(EnqueueOpts {
                queue: "q".into(),
                payload: Bytes::from_static(b"x"),
                run_at: None,
                dedup_key: None,
                priority: 0,
            })
            .await
            .unwrap();
        let c = store
            .claim("q", "w", Duration::from_millis(10), 1)
            .await
            .unwrap();
        assert_eq!(c[0].id, id3);
        tokio::time::sleep(Duration::from_millis(30)).await;
        let n = store.reap(SystemTime::now()).await.unwrap();
        assert!(n >= 1);
        let again = store
            .claim("q", "w2", Duration::from_secs(5), 1)
            .await
            .unwrap();
        assert_eq!(again.len(), 1);
        store
            .fail(&again[0].id, Some(SystemTime::now()))
            .await
            .unwrap();
        let listed = store.list("q", 10).await.unwrap();
        assert!(!listed.is_empty());

        // Higher priority claimed first.
        let low = store
            .enqueue(EnqueueOpts {
                queue: "prio".into(),
                payload: Bytes::from_static(b"low"),
                run_at: None,
                dedup_key: None,
                priority: -100,
            })
            .await
            .unwrap();
        let high = store
            .enqueue(EnqueueOpts {
                queue: "prio".into(),
                payload: Bytes::from_static(b"high"),
                run_at: None,
                dedup_key: None,
                priority: 100,
            })
            .await
            .unwrap();
        let first = store
            .claim("prio", "w", Duration::from_secs(5), 1)
            .await
            .unwrap();
        assert_eq!(first[0].id, high);
        assert_eq!(first[0].priority, 100);
        let second = store
            .claim("prio", "w", Duration::from_secs(5), 1)
            .await
            .unwrap();
        assert_eq!(second[0].id, low);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn memory_conformance() {
        conformance::run(Arc::new(MemoryStore::new())).await;
    }
}
