//! HTTP enqueue integration tests.

use http::Method;
use ruvo_core::{App, Plugin, Request};
use ruvo_tasks::{bearer_guard, TaskBackend, Tasks};
use ruvo_tasks_store::{MemoryStore, TaskStatus, TaskStore};
use std::sync::Arc;

#[tokio::test]
async fn http_enqueue_requires_guard_and_enqueues() {
    let store = Arc::new(MemoryStore::new());

    let mut app = App::new();
    Tasks::new(store.clone())
        .exposed()
        .guard(bearer_guard("secret"))
        .on("job", |_| async { Ok(()) })
        .install(&mut app);

    let body = r#"{"name":"job","payload":{}}"#;
    let ok = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/_tasks/enqueue")
                .header("authorization", "Bearer secret")
                .body(body)
                .build(),
        )
        .await;
    assert_eq!(ok.status_code().as_u16(), 200);

    let forbidden = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/_tasks/enqueue")
                .body(body)
                .build(),
        )
        .await;
    assert_eq!(forbidden.status_code().as_u16(), 403);

    let listed = store.list("default", 10).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, TaskStatus::Pending);
}

#[tokio::test]
async fn backend_enqueue_roundtrip() {
    let store = Arc::new(MemoryStore::new());
    let backend = TaskBackend(store.clone());
    let id = backend
        .enqueue("default", "ping", serde_json::json!({ "n": 1 }))
        .await
        .unwrap();
    assert!(id.starts_with('t'));
    let listed = store.list("default", 10).await.unwrap();
    assert_eq!(listed.len(), 1);
}
