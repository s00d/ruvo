//! Replay tests for Idempotency-Key middleware.

use sova_core::{App, Request, Response, ResponseAssert, TestClient};
use sova_idempotency::Idempotency;
use sova_store::MemoryStore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn replays_second_post_with_same_key() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = Arc::clone(&hits);
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new();
    app.install(
        Idempotency::from_store(store as Arc<dyn sova_store::KvStore>).ttl(Duration::from_secs(60)),
    );
    app.post("/items", move |_req: Request| {
        let hits = Arc::clone(&hits2);
        async move {
            hits.fetch_add(1, Ordering::SeqCst);
            Response::json(&serde_json::json!({ "ok": true })).status(201)
        }
    });

    let client = TestClient::new(app).unwrap();
    let a = client
        .post("/items")
        .header("Idempotency-Key", "k1")
        .json(&serde_json::json!({}))
        .await;
    a.assert_status(201);
    assert_eq!(
        a.headers()
            .get("x-idempotency-replay")
            .and_then(|v| v.to_str().ok()),
        Some("false")
    );

    let b = client
        .post("/items")
        .header("Idempotency-Key", "k1")
        .json(&serde_json::json!({}))
        .await;
    b.assert_status(201);
    assert_eq!(
        b.headers()
            .get("x-idempotency-replay")
            .and_then(|v| v.to_str().ok()),
        Some("true")
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}
