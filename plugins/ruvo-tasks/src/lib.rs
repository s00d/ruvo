//! Background task worker + optional HTTP enqueue for Ruvo.

mod worker;

use bytes::Bytes;
use ruvo_core::{App, Error, IntoResponse, Plugin, Request, Response};
use ruvo_tasks_store::{EnqueueOpts, Task, TaskError, TaskStore};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub(crate) type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub(crate) type Handler = Arc<dyn Fn(Task) -> BoxFuture<Result<(), String>> + Send + Sync>;
pub(crate) type HandlerMap = Arc<HashMap<String, Handler>>;
type Guard = Arc<dyn Fn(&Request) -> bool + Send + Sync>;

use worker::TaskWorker;

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

/// Registers handlers and runs a worker as [`BackgroundService`].
pub struct Tasks {
    store: Arc<dyn TaskStore>,
    queue: String,
    worker_id: String,
    lease: Duration,
    poll: Duration,
    handlers: HashMap<String, Handler>,
    exposed: bool,
    http_guard: Option<Guard>,
    max_attempts: u32,
    retry_base: Duration,
}

impl Tasks {
    pub fn new(store: Arc<dyn TaskStore>) -> Self {
        Self {
            store,
            queue: "default".into(),
            worker_id: format!("w-{}", std::process::id()),
            lease: Duration::from_secs(30),
            poll: Duration::from_millis(200),
            handlers: HashMap::new(),
            exposed: false,
            http_guard: None,
            max_attempts: 5,
            retry_base: Duration::from_secs(5),
        }
    }

    pub fn queue(mut self, q: impl Into<String>) -> Self {
        self.queue = q.into();
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

    /// Fail permanently after this many claim attempts (default 5).
    pub fn max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n.max(1);
        self
    }

    pub fn retry_base(mut self, d: Duration) -> Self {
        self.retry_base = d;
        self
    }

    pub fn on<F, Fut>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn(Task) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        self.handlers
            .insert(name.into(), Arc::new(move |t| Box::pin(f(t))));
        self
    }

    /// Expose `POST /_tasks/enqueue` (requires [`Self::guard`]).
    pub fn exposed(mut self) -> Self {
        self.exposed = true;
        self
    }

    /// Mandatory auth/guard for HTTP enqueue when exposed.
    pub fn guard<F>(mut self, f: F) -> Self
    where
        F: Fn(&Request) -> bool + Send + Sync + 'static,
    {
        self.http_guard = Some(Arc::new(f));
        self
    }
}

impl Plugin for Tasks {
    fn install(self, app: &mut App) {
        let store = self.store.clone();
        app.state(TaskBackend(store.clone()));

        let store_check = store.clone();
        let queue_check = self.queue.clone();
        app.register_check("tasks", move |_state| {
            let store = Arc::clone(&store_check);
            let queue = queue_check.clone();
            async move {
                store
                    .list(&queue, 1)
                    .await
                    .map_err(|e| ruvo_core::Error::Internal(format!("tasks store: {e}")))?;
                Ok(())
            }
        });

        if self.exposed {
            let Some(guard) = self.http_guard.clone() else {
                panic!("Tasks::exposed() requires .guard(...)");
            };
            let queue = self.queue.clone();
            app.post("/_tasks/enqueue", move |mut req: Request| {
                let guard = Arc::clone(&guard);
                let store = Arc::clone(&store);
                let queue = queue.clone();
                async move {
                    if !guard(&req) {
                        return Response::text("forbidden").status(403);
                    }
                    #[derive(serde::Deserialize)]
                    struct Body {
                        name: String,
                        #[serde(default)]
                        payload: serde_json::Value,
                    }
                    let parsed: Body = match req.json().await {
                        Ok(v) => v,
                        Err(e) => return Response::text(e.to_string()).status(400),
                    };
                    let bytes = Bytes::from(
                        serde_json::to_vec(&serde_json::json!({
                            "name": parsed.name,
                            "data": parsed.payload
                        }))
                        .unwrap_or_default(),
                    );
                    match store
                        .enqueue(EnqueueOpts {
                            queue,
                            payload: bytes,
                            run_at: None,
                            dedup_key: None,
                        })
                        .await
                    {
                        Ok(id) => Response::json(&serde_json::json!({ "id": id })),
                        Err(e) => Response::text(e.to_string()).status(500),
                    }
                }
            });
        }

        app.service(TaskWorker {
            store: self.store,
            queue: self.queue,
            worker_id: self.worker_id,
            lease: self.lease,
            poll: self.poll,
            handlers: Arc::new(self.handlers),
            max_attempts: self.max_attempts,
            retry_base: self.retry_base,
        });
    }
}

/// App state handle for enqueue from handlers.
#[derive(Clone)]
pub struct TaskBackend(pub Arc<dyn TaskStore>);

impl TaskBackend {
    pub async fn enqueue(
        &self,
        queue: impl Into<String>,
        name: &str,
        data: serde_json::Value,
    ) -> Result<String, String> {
        let payload = Bytes::from(
            serde_json::to_vec(&serde_json::json!({ "name": name, "data": data }))
                .map_err(|e| e.to_string())?,
        );
        self.0
            .enqueue(EnqueueOpts {
                queue: queue.into(),
                payload,
                run_at: None,
                dedup_key: None,
            })
            .await
            .map_err(|e| e.to_string())
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
    use ruvo_tasks_store::MemoryStore;

    #[tokio::test]
    async fn enqueue_via_backend() {
        let store = Arc::new(MemoryStore::new());
        let backend = TaskBackend(store.clone());
        let id = backend
            .enqueue("default", "ping", serde_json::json!({}))
            .await
            .unwrap();
        assert!(id.starts_with('t'));
        let listed = store.list("default", 10).await.unwrap();
        assert_eq!(listed.len(), 1);
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
        use ruvo_core::BackgroundService;
        use ruvo_tasks_store::TaskStatus;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let store = Arc::new(MemoryStore::new());
        let ran = Arc::new(AtomicUsize::new(0));
        let ran2 = Arc::clone(&ran);

        let mut app = App::new();
        Tasks::new(store.clone())
            .exposed()
            .guard(bearer_guard("secret"))
            .on("job", move |_| {
                let ran2 = Arc::clone(&ran2);
                async move {
                    ran2.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            })
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

        let (tx, shutdown) = ruvo_core::shutdown_channel();
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
            queue: "default".into(),
            worker_id: "test".into(),
            lease: Duration::from_secs(2),
            poll: Duration::from_millis(10),
            handlers: Arc::new(handlers),
            max_attempts: 5,
            retry_base: Duration::from_millis(50),
        });
        let handle = tokio::spawn(worker.run(
            Arc::new(ruvo_core::extend::StateMap::new()),
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
