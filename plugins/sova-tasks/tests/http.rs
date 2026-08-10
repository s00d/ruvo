use http::Method;
use sova_core::{App, Plugin, Request};
use sova_tasks::{bearer_guard, Dispatch, Job, TaskBackend, Tasks};
use sova_tasks_store::{MemoryStore, TaskStore};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn http_enqueue_requires_bearer() {
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new();
    Tasks::new(store.clone())
        .exposed()
        .guard(bearer_guard("secret"))
        .job(Job::new("job", |_| async { Ok(()) }))
        .install(&mut app);

    let forbidden = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/_tasks/enqueue")
                .body(r#"{"name":"job"}"#)
                .build(),
        )
        .await;
    assert_eq!(forbidden.status_code().as_u16(), 403);

    let ok = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/_tasks/enqueue")
                .header("authorization", "Bearer secret")
                .body(r#"{"name":"job","payload":{}}"#)
                .build(),
        )
        .await;
    assert_eq!(ok.status_code().as_u16(), 200);
}

#[tokio::test]
async fn backend_dispatch() {
    let store = Arc::new(MemoryStore::new());
    let backend = TaskBackend {
        store: store.clone(),
        queues: vec!["default".into()],
        job_queues: HashMap::new(),
        job_priorities: HashMap::new(),
        events: None,
    };
    let id = backend
        .dispatch(Dispatch::new("ping").data(serde_json::json!({ "n": 1 })))
        .await
        .unwrap();
    assert!(id.starts_with('t'));
    assert_eq!(store.list("default", 10).await.unwrap().len(), 1);
}
