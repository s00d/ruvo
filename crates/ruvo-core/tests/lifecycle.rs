use http::Method;
use ruvo_core::{App, Error, Request, Response};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[cfg(feature = "testing")]
#[tokio::test]
async fn startup_err_prevents_start() {
    let mut app = App::new();
    app.on_startup(|_state| async { Err(Error::Internal("nope".into())) });
    let err = match app.run_startup().await {
        Err(e) => e,
        Ok(_) => panic!("expected startup error"),
    };
    assert!(matches!(err, Error::Internal(_)));
}

#[cfg(feature = "testing")]
#[tokio::test]
async fn shutdown_hook_runs() {
    let flag = Arc::new(AtomicUsize::new(0));
    let flag2 = Arc::clone(&flag);
    let mut app = App::new();
    app.on_shutdown(move || {
        let flag2 = Arc::clone(&flag2);
        async move {
            flag2.fetch_add(1, Ordering::SeqCst);
        }
    });
    app.run_shutdown().await;
    assert_eq!(flag.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn build_reuses_compiled_server() {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") });
    let server = app.build().unwrap();
    let a = server.handle_request(Method::GET, "/", "").await;
    let b = server.handle_request(Method::GET, "/", "").await;
    assert_eq!(a.body_bytes(), Some(b"ok".as_slice()));
    assert_eq!(b.body_bytes(), Some(b"ok".as_slice()));
}

#[tokio::test]
async fn listen_with_shutdown_stops() {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        app.listen_listener_with_shutdown(listener, async move {
            let _ = rx.await;
        })
        .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let _ = tx.send(());
    let res = tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .expect("server should stop")
        .expect("join");
    assert!(res.is_ok(), "{res:?}");
    let _ = tokio::net::TcpListener::bind(addr).await.unwrap();
}
