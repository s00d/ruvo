//! Concurrent session writes: last writer wins for the whole map (documented contract).

use bytes::Bytes;
use http::Method;
use sova_cookies::CookieLayer;
use sova_core::{App, Request, Response};
use sova_session::{memory_sessions, SessionExt, SessionLayer};
use sova_store::{namespace, KvStore, MemoryStore};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_writes_same_sid_last_writer_wins() {
    let kv = Arc::new(MemoryStore::new());
    let store: Arc<dyn KvStore> = Arc::new(namespace(kv.clone(), "sess"));
    let store_check = store.clone();

    let mut app = App::new();
    app.install(CookieLayer);
    app.install(SessionLayer::new(store.clone()).cookie_name("sid"));
    app.get("/a", |req: Request| async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        req.session().set("a", "1");
        Response::text("a")
    });
    app.get("/b", |req: Request| async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        req.session().set("b", "2");
        Response::text("b")
    });
    let server = Arc::new(app.build().unwrap());

    let seed = server.handle_request(Method::GET, "/a", "").await;
    let sid = seed
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|v| {
            v.strip_prefix("sid=")
                .map(|s| s.split(';').next().unwrap().to_string())
        })
        .expect("sid cookie");

    store_check.remove(&sid).await;

    let s1 = Arc::clone(&server);
    let s2 = Arc::clone(&server);
    let sid1 = sid.clone();
    let sid2 = sid.clone();
    let ha = tokio::spawn(async move {
        let req = Request::builder()
            .method(Method::GET)
            .path("/a")
            .header("cookie", format!("sid={sid1}"))
            .build();
        s1.handle(req).await
    });
    let hb = tokio::spawn(async move {
        let req = Request::builder()
            .method(Method::GET)
            .path("/b")
            .header("cookie", format!("sid={sid2}"))
            .build();
        s2.handle(req).await
    });
    let _ = ha.await.unwrap();
    let _ = hb.await.unwrap();

    let raw = store_check.get(&sid).await.expect("session persisted");
    let text = String::from_utf8_lossy(&raw);
    let has_a = text.contains("a\x001");
    let has_b = text.contains("b\x002");
    assert!(
        (has_a && !has_b) || (has_b && !has_a),
        "expected last-writer-wins single key, got {text:?}"
    );
    let _ = Bytes::new();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn memory_sessions_smoke() {
    let mut app = App::new();
    app.install(CookieLayer);
    app.install(memory_sessions());
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Response::text("ok")
    });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(res.status_code().as_u16(), 200);
}
