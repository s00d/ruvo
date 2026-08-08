//! Concurrent allow() under multi_thread runtime.

use http::Method;
use sova_core::{App, Plugin, Request, Response};
use sova_rate_limit::RateLimit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_allows_respect_max() {
    let mut app = App::new();
    RateLimit::new(10, Duration::from_secs(60)).install(&mut app);
    app.get("/", |_r: Request| async { Response::text("ok") });
    let server = app.build().unwrap();

    let ok = Arc::new(AtomicUsize::new(0));
    let limited = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::with_capacity(100);
    for _ in 0..100 {
        let s = server.clone();
        let ok = Arc::clone(&ok);
        let limited = Arc::clone(&limited);
        handles.push(tokio::spawn(async move {
            let res = s.handle_request(Method::GET, "/", "").await;
            match res.status_code().as_u16() {
                200 => {
                    ok.fetch_add(1, Ordering::SeqCst);
                }
                429 => {
                    limited.fetch_add(1, Ordering::SeqCst);
                }
                other => panic!("unexpected status {other}"),
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    assert_eq!(ok.load(Ordering::SeqCst), 10);
    assert_eq!(limited.load(Ordering::SeqCst), 90);
}

#[tokio::test]
async fn sets_ratelimit_headers() {
    let mut app = App::new();
    RateLimit::new(5, Duration::from_secs(60)).install(&mut app);
    app.get("/", |_r: Request| async { Response::text("ok") });

    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(res.status_code().as_u16(), 200);
    assert_eq!(
        res.headers()
            .get("ratelimit-limit")
            .and_then(|v| v.to_str().ok()),
        Some("5")
    );
    assert!(res.headers().get("ratelimit-remaining").is_some());
    assert!(res.headers().get("ratelimit-reset").is_some());
}
