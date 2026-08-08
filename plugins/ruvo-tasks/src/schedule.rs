//! Recurring schedule producer → [`TaskStore::enqueue`] with slot dedup.

use crate::job::Schedule;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use ruvo_core::extend::{wait_shutdown, BoxFuture, StateMap};
use ruvo_core::{BackgroundService, Shutdown};
use ruvo_tasks_store::{EnqueueOpts, TaskStore};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) struct ScheduledJob {
    pub name: String,
    pub schedule: Schedule,
    pub payload: Value,
    pub queue: String,
    pub priority: i32,
}

pub(crate) struct TaskScheduler {
    pub store: Arc<dyn TaskStore>,
    pub jobs: Vec<ScheduledJob>,
    pub tick: Duration,
}

impl BackgroundService for TaskScheduler {
    fn name(&self) -> &str {
        "tasks-scheduler"
    }

    fn run(
        self: Box<Self>,
        _state: Arc<StateMap>,
        shutdown: Shutdown,
    ) -> BoxFuture<()> {
        Box::pin(async move {
            let mut last: HashMap<String, String> = HashMap::new();
            loop {
                if shutdown.is_triggered() {
                    break;
                }
                self.tick_once(&mut last).await;
                tokio::select! {
                    _ = wait_shutdown(shutdown.clone()) => break,
                    _ = tokio::time::sleep(self.tick) => {}
                }
            }
        })
    }
}

impl TaskScheduler {
    async fn tick_once(&self, last: &mut HashMap<String, String>) {
        let now = SystemTime::now();
        let now_utc: DateTime<Utc> = now.into();
        for job in &self.jobs {
            let slots = due_slots(&job.schedule, now, now_utc, self.tick);
            for slot in slots {
                let dedup = format!("sched:{}:{slot}", job.name);
                if last.get(&job.name).map(|s| s == &dedup).unwrap_or(false) {
                    continue;
                }
                let payload = Bytes::from(
                    serde_json::to_vec(&serde_json::json!({
                        "name": job.name,
                        "data": job.payload,
                    }))
                    .unwrap_or_default(),
                );
                match self
                    .store
                    .enqueue(EnqueueOpts {
                        queue: job.queue.clone(),
                        payload,
                        run_at: Some(now),
                        dedup_key: Some(dedup.clone()),
                        priority: job.priority,
                    })
                    .await
                {
                    Ok(_) => {
                        last.insert(job.name.clone(), dedup);
                    }
                    Err(e) => {
                        tracing::warn!(job = %job.name, error = %e, "schedule enqueue failed");
                    }
                }
            }
        }
    }
}

fn due_slots(
    schedule: &Schedule,
    now: SystemTime,
    now_utc: DateTime<Utc>,
    tick: Duration,
) -> Vec<String> {
    match schedule {
        Schedule::Every(period) => {
            let period_ms = period.as_millis().max(1);
            let now_ms = now
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let slot = now_ms / period_ms;
            vec![slot.to_string()]
        }
        Schedule::Cron(cron) => {
            let window = chrono::Duration::from_std(tick.saturating_mul(2))
                .unwrap_or_else(|_| chrono::Duration::seconds(2));
            let start = now_utc - window;
            let mut out = Vec::new();
            for t in cron.after(&start).take(8) {
                if t <= now_utc {
                    out.push(t.timestamp().to_string());
                } else {
                    break;
                }
            }
            out
        }
    }
}

/// Slot id for an every-schedule at `now` (exported for tests).
pub fn every_slot(period: Duration, now: SystemTime) -> String {
    let period_ms = period.as_millis().max(1);
    let now_ms = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    (now_ms / period_ms).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::parse_cron;
    use ruvo_tasks_store::MemoryStore;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn every_slot_stable_within_period() {
        let p = Duration::from_secs(60);
        let t0 = UNIX_EPOCH + Duration::from_secs(100);
        let t1 = UNIX_EPOCH + Duration::from_secs(119);
        assert_eq!(every_slot(p, t0), every_slot(p, t1));
        let t2 = UNIX_EPOCH + Duration::from_secs(160);
        assert_ne!(every_slot(p, t0), every_slot(p, t2));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn schedule_enqueues_once_per_every_slot() {
        let store = Arc::new(MemoryStore::new());
        let sched = TaskScheduler {
            store: store.clone(),
            jobs: vec![ScheduledJob {
                name: "tick".into(),
                schedule: Schedule::Every(Duration::from_secs(60)),
                payload: serde_json::json!({}),
                queue: "default".into(),
                priority: 0,
            }],
            tick: Duration::from_millis(20),
        };
        let mut last = HashMap::new();
        sched.tick_once(&mut last).await;
        sched.tick_once(&mut last).await;
        let listed = store.list("default", 10).await.unwrap();
        assert_eq!(listed.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn scheduler_service_fires_cron() {
        let store = Arc::new(MemoryStore::new());
        let ran = Arc::new(AtomicUsize::new(0));
        let cron = parse_cron("* * * * * *").unwrap();
        let (tx, shutdown) = ruvo_core::shutdown_channel();
        let service = Box::new(TaskScheduler {
            store: store.clone(),
            jobs: vec![ScheduledJob {
                name: "sec".into(),
                schedule: Schedule::Cron(cron),
                payload: serde_json::json!({}),
                queue: "default".into(),
                priority: 0,
            }],
            tick: Duration::from_millis(50),
        });
        let handle = tokio::spawn(service.run(Arc::new(StateMap::new()), shutdown));

        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            let n = store.list("default", 20).await.unwrap().len();
            if n >= 1 {
                ran.store(n, Ordering::SeqCst);
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        let _ = tx.send(true);
        let _ = handle.await;
        assert!(ran.load(Ordering::SeqCst) >= 1);
    }
}
