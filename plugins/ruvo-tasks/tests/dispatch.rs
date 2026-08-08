//! Extra Tasks plugin coverage: builders, HTTP enqueue options, HttpTaskError.

use http::Method;
use ruvo_core::{App, IntoResponse, Plugin, Request};
use ruvo_tasks_store::{MemoryStore, TaskError, TaskStore};
use ruvo_tasks::{bearer_guard, priority, Dispatch, HttpTaskError, Job, TaskBackend, Tasks};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[tokio::test]
async fn builder_chain_and_meta() {
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new();
    app.install(
        Tasks::new(store)
            .queue("mail")
            .queues(["a", "b"])
            .lease(Duration::from_secs(5))
            .poll_interval(Duration::from_millis(50))
            .max_attempts(3)
            .retry_base(Duration::from_millis(10))
            .scheduler_tick(Duration::from_millis(100))
            .job(
                Job::new("tick", |_| async { Ok(()) })
                    .queue("b")
                    .priority(priority::HIGH)
                    .every(Duration::from_secs(60))
                    .payload(serde_json::json!({ "n": 1 })),
            ),
    );
    assert!(app.has_plugin("tasks"));
    assert!(app
        .installed_plugin_meta()
        .iter()
        .any(|p| p.id == "tasks" && p.meta.name == "Tasks"));
}

#[tokio::test]
async fn http_enqueue_delay_dedup_priority_queue() {
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new();
    Tasks::new(store.clone())
        .queues(["default", "slow"])
        .exposed()
        .guard(bearer_guard("tok"))
        .job(Job::new("job", |_| async { Ok(()) }))
        .install(&mut app);

    let ok = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/_tasks/enqueue")
                .header("authorization", "Bearer tok")
                .body(
                    r#"{"name":"job","payload":{"x":1},"delay_secs":30,"dedup_key":"d1","queue":"slow","priority":50}"#,
                )
                .build(),
        )
        .await;
    assert_eq!(ok.status_code().as_u16(), 200);
    let listed = store.list("slow", 10).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].priority, 50);
    assert!(listed[0].run_at > SystemTime::now());

    let dup = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/_tasks/enqueue")
                .header("authorization", "Bearer tok")
                .body(r#"{"name":"job","dedup_key":"d1","queue":"slow"}"#)
                .build(),
        )
        .await;
    // MemoryStore returns the existing id (200) rather than Duplicate.
    assert_eq!(dup.status_code().as_u16(), 200);
    assert_eq!(store.list("slow", 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn http_enqueue_run_at_and_bad_json() {
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new();
    Tasks::new(store.clone())
        .exposed()
        .guard(bearer_guard("tok"))
        .job(Job::new("job", |_| async { Ok(()) }))
        .install(&mut app);

    let bad = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/_tasks/enqueue")
                .header("authorization", "Bearer tok")
                .body("{")
                .build(),
        )
        .await;
    assert_eq!(bad.status_code().as_u16(), 400);

    let bad_at = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/_tasks/enqueue")
                .header("authorization", "Bearer tok")
                .body(r#"{"name":"job","run_at":"not-rfc3339"}"#)
                .build(),
        )
        .await;
    assert_eq!(bad_at.status_code().as_u16(), 400);

    let at = (chrono::Utc::now() + chrono::Duration::minutes(5)).to_rfc3339();
    let ok = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/_tasks/enqueue")
                .header("authorization", "Bearer tok")
                .body(format!(r#"{{"name":"job","run_at":"{at}"}}"#))
                .build(),
        )
        .await;
    assert_eq!(ok.status_code().as_u16(), 200);
}

#[test]
fn http_task_error_maps_status() {
    let not_found = HttpTaskError(TaskError::NotFound).into_response();
    assert_eq!(not_found.status_code().as_u16(), 404);
    let dup = HttpTaskError(TaskError::Duplicate).into_response();
    assert_eq!(dup.status_code().as_u16(), 409);
    let msg = HttpTaskError(TaskError::Msg("boom".into())).into_response();
    assert_eq!(msg.status_code().as_u16(), 500);

    let err: ruvo_core::Error = HttpTaskError(TaskError::NotFound).into();
    let _ = err;
}

#[tokio::test]
async fn dispatch_at_and_priority_override() {
    let store = Arc::new(MemoryStore::new());
    let backend = TaskBackend {
        store: store.clone(),
        queues: vec!["default".into()],
        job_queues: HashMap::new(),
        job_priorities: HashMap::new(),
    };
    let when = SystemTime::now() + Duration::from_secs(120);
    let id = backend
        .dispatch(
            Dispatch::new("x")
                .at(when)
                .delay(Duration::from_secs(1)) // ignored when at set
                .priority(7)
                .dedup("k"),
        )
        .await
        .unwrap();
    let listed = store.list("default", 10).await.unwrap();
    assert_eq!(listed[0].id, id);
    assert_eq!(listed[0].priority, 7);
}

#[tokio::test]
async fn cron_job_registers_scheduler() {
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new();
    Tasks::new(store)
        .job(Job::new("cronny", |_| async { Ok(()) }).cron("0 * * * *"))
        .install(&mut app);
}
