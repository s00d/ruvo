//! Background task worker + optional scheduler + HTTP dispatch for Sova.

mod console;
mod job;
mod schedule;
mod schedule_toml;
mod worker;

pub use console::{
    ask, confirm, enter_cli, error, info, is_interactive, line, table, warn, ConsoleGuard,
};
pub use job::{parse_cron, priority, Job, Schedule};
pub use schedule::every_slot;

use bytes::Bytes;
use job::Job as JobDef;
use job::priority as prio;
use sova_core::{App, Error, IntoResponse, Plugin, Request, Response};
use sova_tasks_store::{EnqueueOpts, Task, TaskError, TaskStatus, TaskStore};
use schedule::{ScheduledJob, TaskScheduler};
use schedule_toml::{merge_schedules, parse_schedule_toml, schedule_label};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use worker::TaskWorker;

pub(crate) type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub(crate) type Handler = Arc<dyn Fn(Task) -> BoxFuture<Result<(), String>> + Send + Sync>;
pub(crate) type HandlerMap = Arc<HashMap<String, Handler>>;
type Guard = Arc<dyn Fn(&Request) -> bool + Send + Sync>;

/// One registered job (for CLI `tasks list` / `tasks schedule`).
#[derive(Clone, Debug)]
pub struct JobInfo {
    pub name: String,
    pub queue: String,
    pub priority: i32,
    pub schedule: Option<String>,
}

/// App-state registry for CLI and introspection.
#[derive(Clone)]
pub struct TaskRegistry {
    pub handlers: HandlerMap,
    pub jobs: Vec<JobInfo>,
}
/// HTTP-facing wrapper around [`TaskError`] that maps into [`Error::Response`].
#[derive(Debug)]
pub struct HttpTaskError(pub TaskError);

impl From<TaskError> for HttpTaskError {
    fn from(err: TaskError) -> Self {
        Self(err)
    }
}

impl IntoResponse for HttpTaskError {
    fn into_response(self) -> Response {
        match self.0 {
            TaskError::NotFound => Response::text("not found").status(404),
            TaskError::Duplicate => Response::text("duplicate").status(409),
            TaskError::Msg(msg) => Response::text(msg).status(500),
        }
    }
}

impl From<HttpTaskError> for Error {
    fn from(err: HttpTaskError) -> Self {
        Error::Response(Box::new(err.into_response()))
    }
}

/// One-shot (or delayed) enqueue request.
#[derive(Debug, Clone)]
pub struct Dispatch {
    pub name: String,
    pub data: Value,
    pub queue: Option<String>,
    pub run_at: Option<SystemTime>,
    pub delay: Option<Duration>,
    pub dedup_key: Option<String>,
    pub priority: Option<i32>,
}

impl Dispatch {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Value::Object(Default::default()),
            queue: None,
            run_at: None,
            delay: None,
            dedup_key: None,
            priority: None,
        }
    }

    pub fn data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    pub fn queue(mut self, q: impl Into<String>) -> Self {
        self.queue = Some(q.into());
        self
    }

    pub fn at(mut self, when: SystemTime) -> Self {
        self.run_at = Some(when);
        self
    }

    /// Relative delay; ignored when [`Self::at`] is also set.
    pub fn delay(mut self, d: Duration) -> Self {
        self.delay = Some(d);
        self
    }

    pub fn dedup(mut self, key: impl Into<String>) -> Self {
        self.dedup_key = Some(key.into());
        self
    }

    pub fn priority(mut self, p: i32) -> Self {
        self.priority = Some(p);
        self
    }
}

/// Registers jobs and runs a worker (and optional scheduler) as [`BackgroundService`]s.
pub struct Tasks {
    store: Arc<dyn TaskStore>,
    queues: Vec<String>,
    worker_id: String,
    lease: Duration,
    poll: Duration,
    jobs: Vec<JobDef>,
    exposed: bool,
    http_guard: Option<Guard>,
    max_attempts: u32,
    retry_base: Duration,
    scheduler_tick: Duration,
}

impl Tasks {
    pub fn new(store: Arc<dyn TaskStore>) -> Self {
        Self {
            store,
            queues: vec!["default".into()],
            worker_id: format!("w-{}", std::process::id()),
            lease: Duration::from_secs(30),
            poll: Duration::from_millis(200),
            jobs: Vec::new(),
            exposed: false,
            http_guard: None,
            max_attempts: 5,
            retry_base: Duration::from_secs(5),
            scheduler_tick: Duration::from_secs(1),
        }
    }

    /// Single queue (shorthand for [`Self::queues`]).
    pub fn queue(mut self, q: impl Into<String>) -> Self {
        self.queues = vec![q.into()];
        self
    }

    /// Claim order = priority across queues (first is highest).
    pub fn queues<I, S>(mut self, qs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.queues = qs.into_iter().map(Into::into).collect();
        self
    }

    pub fn lease(mut self, d: Duration) -> Self {
        self.lease = d;
        self
    }

    pub fn poll_interval(mut self, d: Duration) -> Self {
        self.poll = d;
        self
    }

    pub fn max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n.max(1);
        self
    }

    pub fn retry_base(mut self, d: Duration) -> Self {
        self.retry_base = d;
        self
    }

    pub fn scheduler_tick(mut self, d: Duration) -> Self {
        self.scheduler_tick = d.max(Duration::from_millis(50));
        self
    }

    pub fn job(mut self, job: JobDef) -> Self {
        self.jobs.push(job);
        self
    }

    pub fn exposed(mut self) -> Self {
        self.exposed = true;
        self
    }

    pub fn guard<F>(mut self, f: F) -> Self
    where
        F: Fn(&Request) -> bool + Send + Sync + 'static,
    {
        self.http_guard = Some(Arc::new(f));
        self
    }

    fn default_queue(&self) -> &str {
        self.queues.first().map(String::as_str).unwrap_or("default")
    }
}

impl Plugin for Tasks {
    fn id(&self) -> &'static str {
        "tasks"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Tasks")
            .description("Job worker, priorities, and optional cron/interval scheduler")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        if self.queues.is_empty() {
            panic!("Tasks::queues must be non-empty");
        }

        let mut job_queues = HashMap::new();
        let mut job_priorities = HashMap::new();
        let mut handlers = HashMap::new();
        let mut scheduled_map: HashMap<String, ScheduledJob> = HashMap::new();
        let default_q = self.default_queue().to_string();

        for job in self.jobs {
            if let Some(ref q) = job.queue {
                job_queues.insert(job.name.clone(), q.clone());
            }
            job_priorities.insert(job.name.clone(), job.priority);
            handlers.insert(job.name.clone(), job.handler);
            if let Some(schedule) = job.schedule {
                let queue = job.queue.clone().unwrap_or_else(|| default_q.clone());
                scheduled_map.insert(
                    job.name.clone(),
                    ScheduledJob {
                        name: job.name,
                        schedule,
                        payload: job.payload,
                        queue,
                        priority: job.priority,
                    },
                );
            }
        }

        let toml_entries = match app.config_doc() {
            Some(doc) => match parse_schedule_toml(doc.as_ref()) {
                Ok(v) => v,
                Err(e) => {
                    let msg = e.clone();
                    app.on_startup(move |_| {
                        let msg = msg.clone();
                        async move {
                            Err(Error::Internal(format!("tasks schedule toml: {msg}")))
                        }
                    });
                    Vec::new()
                }
            },
            None => Vec::new(),
        };

        let scheduled = match merge_schedules(
            scheduled_map,
            toml_entries,
            &handlers,
            &default_q,
            &job_queues,
            &job_priorities,
        ) {
            Ok(v) => v,
            Err(unknown) => {
                let msg = format!(
                    "tasks schedule references unknown job(s): {}",
                    unknown.join(", ")
                );
                let msg2 = msg.clone();
                app.register_audit("tasks-schedule", move |_| {
                    let msg2 = msg2.clone();
                    async move { Err(Error::Internal(msg2)) }
                });
                app.on_startup(move |_| {
                    let msg = msg.clone();
                    async move { Err(Error::Internal(msg)) }
                });
                Vec::new()
            }
        };

        let mut job_infos: Vec<JobInfo> = handlers
            .keys()
            .map(|name| {
                let queue = job_queues
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| default_q.clone());
                let priority = job_priorities.get(name).copied().unwrap_or(prio::NORMAL);
                let schedule = scheduled
                    .iter()
                    .find(|s| &s.name == name)
                    .map(|s| schedule_label(&s.schedule));
                JobInfo {
                    name: name.clone(),
                    queue,
                    priority,
                    schedule,
                }
            })
            .collect();
        job_infos.sort_by(|a, b| a.name.cmp(&b.name));

        let handlers = Arc::new(handlers);
        let registry = TaskRegistry {
            handlers: Arc::clone(&handlers),
            jobs: job_infos,
        };
        app.state(registry.clone());

        let store = self.store.clone();
        let backend = TaskBackend {
            store: store.clone(),
            queues: self.queues.clone(),
            job_queues: job_queues.clone(),
            job_priorities: job_priorities.clone(),
        };
        app.state(backend.clone());

        let store_check = store.clone();
        let queue_check = self.queues[0].clone();
        app.register_check("tasks", move |_state| {
            let store = Arc::clone(&store_check);
            let queue = queue_check.clone();
            async move {
                store
                    .list(&queue, 1)
                    .await
                    .map_err(|e| sova_core::Error::Internal(format!("tasks store: {e}")))?;
                Ok(())
            }
        });

        register_tasks_cli(app, registry);

        if self.exposed {
            let Some(guard) = self.http_guard.clone() else {
                panic!("Tasks::exposed() requires .guard(...)");
            };
            let backend = backend.clone();
            app.post("/_tasks/enqueue", move |mut req: Request| {
                let guard = Arc::clone(&guard);
                let backend = backend.clone();
                async move {
                    if !guard(&req) {
                        return Response::text("forbidden").status(403);
                    }
                    #[derive(serde::Deserialize)]
                    struct Body {
                        name: String,
                        #[serde(default)]
                        payload: Value,
                        #[serde(default)]
                        run_at: Option<String>,
                        #[serde(default)]
                        delay_secs: Option<u64>,
                        #[serde(default)]
                        dedup_key: Option<String>,
                        #[serde(default)]
                        queue: Option<String>,
                        #[serde(default)]
                        priority: Option<i32>,
                    }
                    let parsed: Body = match req.json().await {
                        Ok(v) => v,
                        Err(e) => return Response::text(e.to_string()).status(400),
                    };
                    let mut d = Dispatch::new(parsed.name).data(parsed.payload);
                    if let Some(q) = parsed.queue {
                        d = d.queue(q);
                    }
                    if let Some(s) = parsed.run_at {
                        match chrono::DateTime::parse_from_rfc3339(&s) {
                            Ok(dt) => {
                                d = d.at(SystemTime::from(dt.with_timezone(&chrono::Utc)));
                            }
                            Err(e) => {
                                return Response::text(format!("run_at: {e}")).status(400);
                            }
                        }
                    } else if let Some(secs) = parsed.delay_secs {
                        d = d.delay(Duration::from_secs(secs));
                    }
                    if let Some(k) = parsed.dedup_key {
                        d = d.dedup(k);
                    }
                    if let Some(p) = parsed.priority {
                        d = d.priority(p);
                    }
                    match backend.dispatch(d).await {
                        Ok(id) => Response::json(&serde_json::json!({ "id": id })),
                        Err(e) => Response::text(e.to_string()).status(500),
                    }
                }
            });
        }

        if !scheduled.is_empty() {
            app.service(TaskScheduler {
                store: store.clone(),
                jobs: scheduled,
                tick: self.scheduler_tick,
            });
        }

        app.service(TaskWorker {
            store: self.store,
            queues: self.queues,
            worker_id: self.worker_id,
            lease: self.lease,
            poll: self.poll,
            handlers,
            max_attempts: self.max_attempts,
            retry_base: self.retry_base,
        });
    }
}

fn register_tasks_cli(app: &mut App, registry: TaskRegistry) {
    app.register_cli("tasks", move |state, args| {
        let registry = state
            .get::<TaskRegistry>()
            .map(|a| (*a).clone())
            .unwrap_or_else(|| registry.clone());
        async move {
            let sub = args.first().map(String::as_str).unwrap_or("list");
            match sub {
                "list" | "" => {
                    let rows: Vec<Vec<String>> = registry
                        .jobs
                        .iter()
                        .map(|j| {
                            vec![
                                j.name.clone(),
                                j.queue.clone(),
                                j.priority.to_string(),
                                j.schedule.clone().unwrap_or_else(|| "-".into()),
                            ]
                        })
                        .collect();
                    // Always print for list (CLI already interactive at process level).
                    println!(
                        "{:<24} {:<12} {:<8} SCHEDULE",
                        "NAME", "QUEUE", "PRIO"
                    );
                    for r in &rows {
                        println!(
                            "{:<24} {:<12} {:<8} {}",
                            r[0], r[1], r[2], r[3]
                        );
                    }
                    if rows.is_empty() {
                        println!("(no jobs registered)");
                    }
                    Ok(())
                }
                "schedule" => {
                    let scheduled: Vec<_> = registry
                        .jobs
                        .iter()
                        .filter(|j| j.schedule.is_some())
                        .collect();
                    if scheduled.is_empty() {
                        println!("(no scheduled jobs)");
                    } else {
                        for j in scheduled {
                            println!(
                                "{:<24} {}  queue={}  priority={}",
                                j.name,
                                j.schedule.as_deref().unwrap_or("-"),
                                j.queue,
                                j.priority
                            );
                        }
                    }
                    Ok(())
                }
                "run" => {
                    let name = args
                        .get(1)
                        .cloned()
                        .ok_or_else(|| Error::Internal("usage: tasks run NAME [--json '{…}']".into()))?;
                    let mut data = Value::Object(Default::default());
                    if let Some(idx) = args.iter().position(|a| a == "--json") {
                        let raw = args.get(idx + 1).ok_or_else(|| {
                            Error::Internal("tasks run: --json requires a value".into())
                        })?;
                        data = serde_json::from_str(raw).map_err(|e| {
                            Error::Internal(format!("tasks run --json: {e}"))
                        })?;
                    }
                    let Some(handler) = registry.handlers.get(&name) else {
                        return Err(Error::Internal(format!(
                            "unknown job `{name}` (try `tasks list`)"
                        )));
                    };
                    let payload = Bytes::from(
                        serde_json::to_vec(&serde_json::json!({
                            "name": name,
                            "data": data,
                        }))
                        .map_err(|e| Error::Internal(e.to_string()))?,
                    );
                    let task = Task {
                        id: format!("cli-{}", name),
                        queue: registry
                            .jobs
                            .iter()
                            .find(|j| j.name == name)
                            .map(|j| j.queue.clone())
                            .unwrap_or_else(|| "default".into()),
                        payload,
                        run_at: SystemTime::now(),
                        attempts: 1,
                        lease_until: None,
                        dedup_key: None,
                        status: TaskStatus::Running,
                        worker: Some("cli".into()),
                        priority: registry
                            .jobs
                            .iter()
                            .find(|j| j.name == name)
                            .map(|j| j.priority)
                            .unwrap_or(0),
                    };
                    let _guard = enter_cli();
                    handler(task)
                        .await
                        .map_err(|e| Error::Internal(format!("job `{name}` failed: {e}")))?;
                    println!("ok {name}");
                    Ok(())
                }
                "help" | "-h" | "--help" => {
                    println!("usage:");
                    println!("  tasks list");
                    println!("  tasks schedule");
                    println!("  tasks run NAME [--json '{{…}}']");
                    Ok(())
                }
                other => Err(Error::Internal(format!(
                    "unknown tasks subcommand `{other}` (list|schedule|run)"
                ))),
            }
        }
    });
}

/// App state handle for dispatch from request handlers.
#[derive(Clone)]
pub struct TaskBackend {
    pub store: Arc<dyn TaskStore>,
    pub queues: Vec<String>,
    pub job_queues: HashMap<String, String>,
    pub job_priorities: HashMap<String, i32>,
}

impl TaskBackend {
    pub async fn dispatch(&self, d: Dispatch) -> Result<String, TaskError> {
        let started = std::time::Instant::now();
        let name = d.name.clone();
        let queue = d
            .queue
            .clone()
            .or_else(|| self.job_queues.get(&d.name).cloned())
            .unwrap_or_else(|| {
                self.queues
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "default".into())
            });
        let priority = d
            .priority
            .or_else(|| self.job_priorities.get(&d.name).copied())
            .unwrap_or(prio::NORMAL);
        let run_at = match (d.run_at, d.delay) {
            (Some(at), _) => Some(at),
            (None, Some(delay)) => Some(SystemTime::now() + delay),
            (None, None) => None,
        };
        let payload = Bytes::from(
            serde_json::to_vec(&serde_json::json!({
                "name": d.name,
                "data": d.data,
            }))
            .map_err(|e| TaskError::Msg(e.to_string()))?,
        );
        let queue_for_log = queue.clone();
        let id = self
            .store
            .enqueue(EnqueueOpts {
                queue,
                payload,
                run_at,
                dedup_key: d.dedup_key,
                priority,
            })
            .await?;
        tracing::debug!(
            target: "sova.tasks",
            name = %name,
            queue = %queue_for_log,
            id = %id,
            status = "enqueued",
            duration_ms = started.elapsed().as_secs_f64() * 1000.0,
            request_id = sova_core::current_request_id().as_deref().unwrap_or(""),
            "sova.tasks enqueue"
        );
        Ok(id)
    }
}

/// Require `Authorization: Bearer <token>` for task HTTP routes.
pub fn bearer_guard(token: impl Into<String>) -> impl Fn(&Request) -> bool + Send + Sync + 'static {
    let token = token.into();
    move |req: &Request| {
        req.header("authorization")
            .map(|v| v == format!("Bearer {token}"))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sova_tasks_store::MemoryStore;

    #[tokio::test]
    async fn dispatch_via_backend() {
        let store = Arc::new(MemoryStore::new());
        let backend = TaskBackend {
            store: store.clone(),
            queues: vec!["default".into()],
            job_queues: HashMap::new(),
            job_priorities: HashMap::new(),
        };
        let id = backend
            .dispatch(Dispatch::new("ping").data(serde_json::json!({})))
            .await
            .unwrap();
        assert!(id.starts_with('t'));
        assert_eq!(store.list("default", 10).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn dispatch_uses_job_defaults_and_delay() {
        let store = Arc::new(MemoryStore::new());
        let mut job_queues = HashMap::new();
        job_queues.insert("mail".into(), "mailer".into());
        let mut job_priorities = HashMap::new();
        job_priorities.insert("mail".into(), prio::LOW);
        let backend = TaskBackend {
            store: store.clone(),
            queues: vec!["default".into(), "mailer".into()],
            job_queues,
            job_priorities,
        };
        let id = backend
            .dispatch(Dispatch::new("mail").delay(Duration::from_secs(60)))
            .await
            .unwrap();
        let listed = store.list("mailer", 10).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, id);
        assert_eq!(listed[0].priority, prio::LOW);
        assert!(listed[0].run_at > SystemTime::now());
    }

    #[test]
    #[should_panic(expected = "Tasks::exposed() requires .guard")]
    fn exposed_without_guard_panics_on_install() {
        let store = Arc::new(MemoryStore::new());
        let mut app = App::new();
        Tasks::new(store).exposed().install(&mut app);
    }

    #[tokio::test]
    async fn http_enqueue_then_worker_runs_handler() {
        use http::Method;
        use sova_core::BackgroundService;
        use sova_tasks_store::TaskStatus;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let store = Arc::new(MemoryStore::new());
        let ran = Arc::new(AtomicUsize::new(0));
        let ran2 = Arc::clone(&ran);

        let mut app = App::new();
        Tasks::new(store.clone())
            .exposed()
            .guard(bearer_guard("secret"))
            .job(Job::new("job", move |_| {
                let ran2 = Arc::clone(&ran2);
                async move {
                    ran2.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            }))
            .install(&mut app);

        let body = r#"{"name":"job","payload":{}}"#;
        let res = app
            .handle(
                Request::builder()
                    .method(Method::POST)
                    .path("/_tasks/enqueue")
                    .header("authorization", "Bearer secret")
                    .body(body)
                    .build(),
            )
            .await;
        assert_eq!(res.status_code().as_u16(), 200);

        let (tx, shutdown) = sova_core::shutdown_channel();
        let mut handlers = HashMap::new();
        let ran3 = Arc::clone(&ran);
        handlers.insert(
            "job".into(),
            Arc::new(move |_t| {
                let ran3 = Arc::clone(&ran3);
                Box::pin(async move {
                    ran3.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }) as crate::BoxFuture<Result<(), String>>
            }) as Handler,
        );
        let worker = Box::new(TaskWorker {
            store: store.clone(),
            queues: vec!["default".into()],
            worker_id: "test".into(),
            lease: Duration::from_secs(2),
            poll: Duration::from_millis(10),
            handlers: Arc::new(handlers),
            max_attempts: 5,
            retry_base: Duration::from_millis(50),
        });
        let handle = tokio::spawn(worker.run(
            Arc::new(sova_core::extend::StateMap::new()),
            shutdown,
        ));

        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            if ran.load(Ordering::SeqCst) >= 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(ran.load(Ordering::SeqCst), 1);
        let listed = store.list("default", 10).await.unwrap();
        assert_eq!(listed[0].status, TaskStatus::Done);

        let _ = tx.send(true);
        let _ = handle.await;
    }
}
