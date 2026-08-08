//! Memory TaskStore: enqueue / claim / ack (complete).

use bytes::Bytes;
use sova_tasks_store::{EnqueueOpts, MemoryStore, TaskStatus, TaskStore};
use std::time::Duration;

#[tokio::test]
async fn enqueue_claim_ack() {
    let store = MemoryStore::new();
    let id = store
        .enqueue(EnqueueOpts {
            queue: "default".into(),
            payload: Bytes::from_static(b"hello"),
            run_at: None,
            dedup_key: None,
            priority: 0,
        })
        .await
        .unwrap();

    let claimed = store
        .claim("default", "worker-1", Duration::from_secs(30), 10)
        .await
        .unwrap();
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].id, id);
    assert_eq!(claimed[0].status, TaskStatus::Running);
    assert_eq!(claimed[0].payload.as_ref(), b"hello");

    store.complete(&id).await.unwrap();

    let listed = store.list("default", 10).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, TaskStatus::Done);

    let empty = store
        .claim("default", "worker-2", Duration::from_secs(5), 10)
        .await
        .unwrap();
    assert!(empty.is_empty());
}
