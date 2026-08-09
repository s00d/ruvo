use sova_core::{App, Request, Response, ResponseAssert, TestClient};
use sova_response_cache::ResponseCache;
use sova_store::MemoryStore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn caches_get_200() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = Arc::clone(&hits);
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new();
    app.install(
        ResponseCache::new(store as Arc<dyn sova_store::KvStore>).ttl(Duration::from_secs(30)),
    );
    app.get("/public", move |_req: Request| {
        let hits = Arc::clone(&hits2);
        async move {
            hits.fetch_add(1, Ordering::SeqCst);
            Response::json(&serde_json::json!({ "n": 1 }))
        }
    });

    let client = TestClient::new(app).unwrap();
    let a = client.get("/public").await;
    a.assert_status(200);
    assert_eq!(a.headers().get("x-cache").and_then(|v| v.to_str().ok()), Some("MISS"));
    let b = client.get("/public").await;
    b.assert_status(200);
    assert_eq!(b.headers().get("x-cache").and_then(|v| v.to_str().ok()), Some("HIT"));
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn skips_cookie_by_default() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = Arc::clone(&hits);
    let store = Arc::new(MemoryStore::new());
    let mut app = App::new();
    app.install(ResponseCache::new(store as Arc<dyn sova_store::KvStore>));
    app.get("/public", move |_req: Request| {
        let hits = Arc::clone(&hits2);
        async move {
            hits.fetch_add(1, Ordering::SeqCst);
            Response::text("ok")
        }
    });
    let client = TestClient::new(app).unwrap();
    let _ = client.get("/public").header("cookie", "a=1").await;
    let _ = client.get("/public").header("cookie", "a=1").await;
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}
