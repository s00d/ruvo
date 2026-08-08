use crate::{Handler, HandlerMap};
use sova_core::extend::{wait_shutdown, BoxFuture, StateMap};
use sova_core::{BackgroundService, Shutdown};
use sova_tasks_store::{Task, TaskStore};
use std::sync::Arc;
use std::time::Duration;

pub(crate) struct TaskWorker {
    pub store: Arc<dyn TaskStore>,
    pub queues: Vec<String>,
    pub worker_id: String,
    pub lease: Duration,
    pub poll: Duration,
    pub handlers: HandlerMap,
    pub max_attempts: u32,
    pub retry_base: Duration,
}

impl BackgroundService for TaskWorker {
    fn name(&self) -> &str {
        "tasks-worker"
    }

    fn run(
        self: Box<Self>,
        _state: Arc<StateMap>,
        shutdown: Shutdown,
    ) -> BoxFuture<()> {
        Box::pin(async move {
            loop {
                if shutdown.is_triggered() {
                    break;
                }
                let _ = self.store.reap(std::time::SystemTime::now()).await;
                let mut got = false;
                for q in &self.queues {
                    match self
                        .store
                        .claim(q, &self.worker_id, self.lease, 1)
                        .await
                    {
                        Ok(batch) if !batch.is_empty() => {
                            got = true;
                            for task in batch {
                                Self::dispatch_one(
                                    &self.store,
                                    &self.handlers,
                                    task,
                                    self.lease,
                                    self.max_attempts,
                                    self.retry_base,
                                )
                                .await;
                            }
                            break;
                        }
                        _ => {}
                    }
                }
                if !got {
                    tokio::select! {
                        _ = wait_shutdown(shutdown.clone()) => break,
                        _ = tokio::time::sleep(self.poll) => {}
                    }
                }
            }
        })
    }
}

impl TaskWorker {
    async fn dispatch_one(
        store: &Arc<dyn TaskStore>,
        handlers: &HandlerMap,
        task: Task,
        lease: Duration,
        max_attempts: u32,
        retry_base: Duration,
    ) {
        let name = task_name(&task);
        let Some(h) = handlers.get(&name) else {
            tracing::warn!(%name, "no handler");
            let _ = store.fail(&task.id, None).await;
            return;
        };

        match run_with_heartbeat(store, &task, lease, h).await {
            Ok(()) => {
                let _ = store.complete(&task.id).await;
            }
            Err(e) => {
                tracing::warn!(id = %task.id, error = %e, "task failed");
                let retry_at = if task.attempts >= max_attempts {
                    None
                } else {
                    let mult = task.attempts.max(1);
                    Some(
                        std::time::SystemTime::now()
                            + retry_base.saturating_mul(mult),
                    )
                };
                let _ = store.fail(&task.id, retry_at).await;
            }
        }
    }
}

pub(crate) fn task_name(task: &Task) -> String {
    serde_json::from_slice::<serde_json::Value>(&task.payload)
        .ok()
        .and_then(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
}

pub(crate) async fn run_with_heartbeat(
    store: &Arc<dyn TaskStore>,
    task: &Task,
    lease: Duration,
    handler: &Handler,
) -> Result<(), String> {
    let id = task.id.clone();
    let store_hb = Arc::clone(store);
    let heartbeat_every = (lease / 2).max(Duration::from_millis(50));
    let handler_fut = handler(task.clone());
    tokio::pin!(handler_fut);
    loop {
        tokio::select! {
            res = &mut handler_fut => return res,
            _ = tokio::time::sleep(heartbeat_every) => {
                if store_hb.heartbeat(&id, lease).await.is_err() {
                    break;
                }
            }
        }
    }
    handler_fut.await
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use sova_tasks_store::{EnqueueOpts, MemoryStore, TaskStatus};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    fn payload(name: &str) -> Bytes {
        Bytes::from(
            serde_json::to_vec(&serde_json::json!({ "name": name, "data": {} }))
                .unwrap(),
        )
    }

    async fn enqueue(store: &MemoryStore, queue: &str, name: &str) -> String {
        store
            .enqueue(EnqueueOpts {
                queue: queue.into(),
                payload: payload(name),
                run_at: None,
                dedup_key: None,
                priority: 0,
            })
            .await
            .unwrap()
    }

    fn spawn_worker(
        store: Arc<MemoryStore>,
        worker_id: &str,
        handlers: HandlerMap,
        lease: Duration,
        poll: Duration,
        max_attempts: u32,
    ) -> (sova_core::ShutdownSender, tokio::task::JoinHandle<()>) {
        let (tx, shutdown) = sova_core::shutdown_channel();
        let worker = Box::new(TaskWorker {
            store,
            queues: vec!["default".into()],
            worker_id: worker_id.into(),
            lease,
            poll,
            handlers,
            max_attempts,
            retry_base: Duration::from_millis(50),
        });
        let handle = tokio::spawn(worker.run(Arc::new(StateMap::new()), shutdown));
        (tx, handle)
    }

    async fn wait_until<F, Fut>(timeout: Duration, mut f: F)
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        while tokio::time::Instant::now() < deadline {
            if f().await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("timeout waiting for condition");
    }

    async fn pending_count(store: &MemoryStore) -> usize {
        store
            .list("default", 500)
            .await
            .map(|v| {
                v.iter()
                    .filter(|t| t.status != TaskStatus::Done && t.status != TaskStatus::Failed)
                    .count()
            })
            .unwrap_or(500)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn two_workers_process_100_tasks_once_each() {
        let store = Arc::new(MemoryStore::new());
        for _ in 0..100 {
            enqueue(&store, "default", "job").await;
        }

        let runs = Arc::new(Mutex::new(HashMap::<String, u32>::new()));
        let runs2 = Arc::clone(&runs);
        let handler: Handler = Arc::new(move |t| {
            let runs2 = Arc::clone(&runs2);
            Box::pin(async move {
                let mut g = runs2.lock().unwrap();
                *g.entry(t.id.clone()).or_insert(0) += 1;
                Ok(())
            })
        });
        let handlers = Arc::new({
            let mut m = HashMap::new();
            m.insert("job".into(), handler);
            m
        });

        let (tx1, h1) = spawn_worker(
            Arc::clone(&store),
            "w1",
            Arc::clone(&handlers),
            Duration::from_secs(5),
            Duration::from_millis(10),
            5,
        );
        let (tx2, h2) = spawn_worker(
            Arc::clone(&store),
            "w2",
            handlers,
            Duration::from_secs(5),
            Duration::from_millis(10),
            5,
        );

        wait_until(Duration::from_secs(5), || async {
            pending_count(&store).await == 0
        })
        .await;

        let _ = tx1.send(true);
        let _ = tx2.send(true);
        let _ = h1.await;
        let _ = h2.await;

        let counts = runs.lock().unwrap();
        assert_eq!(counts.len(), 100);
        for c in counts.values() {
            assert_eq!(*c, 1, "each task must run exactly once");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn crashed_worker_task_reclaimed_after_lease() {
        let store = Arc::new(MemoryStore::new());
        enqueue(&store, "default", "slow").await;

        let started = Arc::new(AtomicUsize::new(0));
        let finished = Arc::new(AtomicUsize::new(0));
        let started2 = Arc::clone(&started);
        let finished2 = Arc::clone(&finished);
        let handler: Handler = Arc::new(move |_t| {
            let started2 = Arc::clone(&started2);
            let finished2 = Arc::clone(&finished2);
            Box::pin(async move {
                started2.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(500)).await;
                finished2.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });
        let handlers = Arc::new({
            let mut m = HashMap::new();
            m.insert("slow".into(), handler);
            m
        });

        let lease = Duration::from_millis(120);
        let (tx1, h1) = spawn_worker(
            Arc::clone(&store),
            "w1",
            Arc::clone(&handlers),
            lease,
            Duration::from_millis(10),
            5,
        );

        tokio::time::sleep(Duration::from_millis(40)).await;
        h1.abort();
        let _ = tx1.send(true);

        tokio::time::sleep(lease + Duration::from_millis(80)).await;
        let _ = store.reap(std::time::SystemTime::now()).await;

        let (tx2, h2) = spawn_worker(
            Arc::clone(&store),
            "w2",
            handlers,
            lease,
            Duration::from_millis(10),
            5,
        );

        wait_until(Duration::from_secs(5), || async {
            finished.load(Ordering::SeqCst) >= 1
        })
        .await;

        let _ = tx2.send(true);
        let _ = h2.await;
        assert!(started.load(Ordering::SeqCst) >= 1);
        assert_eq!(finished.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn heartbeat_keeps_long_task_from_reclaim() {
        let store = Arc::new(MemoryStore::new());
        enqueue(&store, "default", "long").await;

        let concurrent = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));
        let c1 = Arc::clone(&concurrent);
        let m1 = Arc::clone(&max_concurrent);
        let handler: Handler = Arc::new(move |_t| {
            let c1 = Arc::clone(&c1);
            let m1 = Arc::clone(&m1);
            Box::pin(async move {
                let n = c1.fetch_add(1, Ordering::SeqCst) + 1;
                m1.fetch_max(n, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(350)).await;
                c1.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            })
        });
        let handlers = Arc::new({
            let mut m = HashMap::new();
            m.insert("long".into(), handler);
            m
        });

        let lease = Duration::from_millis(150);
        let (tx1, h1) = spawn_worker(
            Arc::clone(&store),
            "w1",
            Arc::clone(&handlers),
            lease,
            Duration::from_millis(10),
            5,
        );
        let (tx2, h2) = spawn_worker(
            Arc::clone(&store),
            "w2",
            handlers,
            lease,
            Duration::from_millis(10),
            5,
        );

        // Past lease expiry: task must stay Running (heartbeat) until handler finishes.
        tokio::time::sleep(lease + Duration::from_millis(80)).await;
        let mid = store.list("default", 10).await.unwrap();
        assert_eq!(mid.len(), 1);
        assert_eq!(mid[0].status, TaskStatus::Running);

        wait_until(Duration::from_secs(3), || async {
            store
                .list("default", 10)
                .await
                .ok()
                .map(|v| v.iter().any(|t| t.status == TaskStatus::Done))
                .unwrap_or(false)
        })
        .await;

        let _ = tx1.send(true);
        let _ = tx2.send(true);
        let _ = h1.await;
        let _ = h2.await;
        assert_eq!(max_concurrent.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn dedup_key_enqueues_once() {
        let store = Arc::new(MemoryStore::new());
        let id1 = store
            .enqueue(EnqueueOpts {
                queue: "default".into(),
                payload: payload("x"),
                run_at: None,
                dedup_key: Some("slot".into()),
                priority: 0,
            })
            .await
            .unwrap();
        let id2 = store
            .enqueue(EnqueueOpts {
                queue: "default".into(),
                payload: payload("x"),
                run_at: None,
                dedup_key: Some("slot".into()),
                priority: 0,
            })
            .await
            .unwrap();
        assert_eq!(id1, id2);

        let runs = Arc::new(AtomicUsize::new(0));
        let runs2 = Arc::clone(&runs);
        let handler: Handler = Arc::new(move |_| {
            let runs2 = Arc::clone(&runs2);
            Box::pin(async move {
                runs2.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });
        let handlers = Arc::new({
            let mut m = HashMap::new();
            m.insert("x".into(), handler);
            m
        });

        let (tx, h) = spawn_worker(
            store.clone(),
            "w1",
            handlers,
            Duration::from_secs(2),
            Duration::from_millis(10),
            5,
        );
        wait_until(Duration::from_secs(2), || async {
            runs.load(Ordering::SeqCst) >= 1
        })
        .await;
        let _ = tx.send(true);
        let _ = h.await;
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn max_attempts_marks_failed() {
        let store = Arc::new(MemoryStore::new());
        let id = enqueue(&store, "default", "fail").await;

        let handler: Handler = Arc::new(|_| Box::pin(async { Err("nope".into()) }));
        let handlers = Arc::new({
            let mut m = HashMap::new();
            m.insert("fail".into(), handler);
            m
        });

        let (tx, h) = spawn_worker(
            store.clone(),
            "w1",
            handlers,
            Duration::from_secs(2),
            Duration::from_millis(10),
            2,
        );

        wait_until(Duration::from_secs(3), || async {
            store
                .list("default", 10)
                .await
                .ok()
                .and_then(|v| v.into_iter().find(|t| t.id == id))
                .map(|t| t.status == TaskStatus::Failed)
                .unwrap_or(false)
        })
        .await;

        let _ = tx.send(true);
        let _ = h.await;
        let task = store
            .list("default", 10)
            .await
            .unwrap()
            .into_iter()
            .find(|t| t.id == id)
            .unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert!(task.attempts >= 2);
    }

    #[tokio::test]
    async fn shutdown_waits_for_in_flight_handler() {
        let store = Arc::new(MemoryStore::new());
        enqueue(&store, "default", "slow").await;

        let started = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(AtomicUsize::new(0));
        let started2 = Arc::clone(&started);
        let done2 = Arc::clone(&done);
        let handler: Handler = Arc::new(move |_| {
            let started2 = Arc::clone(&started2);
            let done2 = Arc::clone(&done2);
            Box::pin(async move {
                started2.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(200)).await;
                done2.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });
        let handlers = Arc::new({
            let mut m = HashMap::new();
            m.insert("slow".into(), handler);
            m
        });

        let (tx, h) = spawn_worker(
            store.clone(),
            "w1",
            handlers,
            Duration::from_secs(2),
            Duration::from_millis(10),
            5,
        );

        wait_until(Duration::from_secs(2), || async {
            started.load(Ordering::SeqCst) >= 1
        })
        .await;
        let _ = tx.send(true);
        let _ = h.await;
        assert_eq!(done.load(Ordering::SeqCst), 1);
    }
}
