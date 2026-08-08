//! Concurrent dispatch / accept-loop behaviour (multi_thread).

mod common;

use common::{http_get, LiveServer};
use http::Method;
use sova_core::{App, Request, Response};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_sleep_handlers_overlap() {
    let mut app = App::new();
    app.get("/sleep", |_r: Request| async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Response::text("ok")
    });
    let server = app.build().unwrap();

    let start = Instant::now();
    let mut handles = Vec::with_capacity(50);
    for _ in 0..50 {
        let s = server.clone();
        handles.push(tokio::spawn(async move {
            s.handle_request(Method::GET, "/sleep", "").await
        }));
    }
    for h in handles {
        let res = h.await.unwrap();
        assert_eq!(res.status_code().as_u16(), 200);
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(800),
        "expected overlapping sleeps, got {elapsed:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_handle_is_send_sync_under_load() {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") });
    let server = app.build().unwrap();

    let mut handles = Vec::with_capacity(100);
    for i in 0..100 {
        let s = server.clone();
        handles.push(tokio::spawn(async move {
            s.handle_request(Method::GET, "/", "").await.body_bytes()
                == Some(b"ok".as_slice())
                && i < 1000
        }));
    }
    for h in handles {
        assert!(h.await.unwrap());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_drains_in_flight_requests() {
    let done = Arc::new(AtomicUsize::new(0));
    let done2 = Arc::clone(&done);

    let mut app = App::new();
    app.get("/slow", move |_r: Request| {
        let done2 = Arc::clone(&done2);
        async move {
            tokio::time::sleep(Duration::from_millis(150)).await;
            done2.fetch_add(1, Ordering::SeqCst);
            Response::text("ok")
        }
    });

    let live = LiveServer::spawn(app).await;
    let addr = live.addr;

    let mut handles = Vec::with_capacity(20);
    for _ in 0..20 {
        handles.push(tokio::spawn(async move { http_get(addr, "/slow").await }));
    }
    tokio::time::sleep(Duration::from_millis(40)).await;
    live.shutdown().await;

    for h in handles {
        let (status, _) = h.await.unwrap();
        assert_eq!(status, 200);
    }
    assert_eq!(done.load(Ordering::SeqCst), 20);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn max_connections_rejects_overflow() {
    let mut app = App::new();
    app.max_connections(2);
    app.get("/hold", |_r: Request| async {
        tokio::time::sleep(Duration::from_millis(400)).await;
        Response::text("held")
    });
    app.get("/ping", |_r: Request| async { Response::text("pong") });

    let live = LiveServer::spawn(app).await;
    let addr = live.addr;

    let hold_a = tokio::spawn(async move { http_get(addr, "/hold").await });
    let hold_b = tokio::spawn(async move { http_get(addr, "/hold").await });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut rejected = 0usize;
    let mut overflow = Vec::new();
    for _ in 0..5 {
        overflow.push(tokio::spawn(async move { http_get(addr, "/hold").await }));
    }
    for h in overflow {
        let rejected_one = !matches!(
            tokio::time::timeout(Duration::from_millis(200), h).await,
            Ok(Ok((200, _)))
        );
        if rejected_one {
            rejected += 1;
        }
    }
    assert!(rejected >= 3, "expected most overflow rejected, got rejected={rejected}");

    let (a, _) = hold_a.await.unwrap();
    let (b, _) = hold_b.await.unwrap();
    assert_eq!(a, 200);
    assert_eq!(b, 200);

    let (status, body) = http_get(addr, "/ping").await;
    assert_eq!(status, 200);
    assert_eq!(body.as_ref(), b"pong");

    live.shutdown().await;
}
