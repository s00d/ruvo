//! Concurrent session writes: last writer wins for the whole map (documented contract).

use http::Method;
use ruvo_core::{App, Plugin, Request, Response};
use ruvo_session::{memory_sessions, MemoryStore, SessionExt, SessionLayer, SessionStore};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_writes_same_sid_last_writer_wins() {
    let store = MemoryStore::new();
    let store_check = store.clone();

    let mut app = App::new();
    SessionLayer::new(store)
        .cookie_name("sid")
        .install(&mut app);
    app.get("/a", |req: Request| async move {
        // Slow enough that both requests load before either persists.
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

    // Seed a shared sid via first request.
    let seed = server
        .handle_request(Method::GET, "/a", "")
        .await;
    let sid = seed
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|v| v.strip_prefix("sid=").map(|s| s.split(';').next().unwrap().to_string()))
        .expect("sid cookie");

    // Clear store so both subsequent requests start from empty data for that sid.
    store_check.remove(&sid);

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

    let data = store_check.get(&sid).expect("session persisted");
    // Contract: whole-map replace → only one key survives.
    assert!(
        (data.contains_key("a") && !data.contains_key("b"))
            || (data.contains_key("b") && !data.contains_key("a")),
        "expected last-writer-wins single key, got {data:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn memory_sessions_smoke() {
    let mut app = App::new();
    memory_sessions().install(&mut app);
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Response::text("ok")
    });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(res.status_code().as_u16(), 200);
}
