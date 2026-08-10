use http::Method;
use sova_core::extend::Bind;
#[cfg(feature = "testing")]
use sova_core::Error;
use sova_core::{App, Request, Response};
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

#[cfg(feature = "testing")]
#[tokio::test]
async fn run_startup_is_non_destructive() {
    let mut app = App::new();
    let n = Arc::new(AtomicUsize::new(0));
    let n2 = Arc::clone(&n);
    app.on_startup(move |_s| {
        let n2 = Arc::clone(&n2);
        async move {
            n2.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });
    app.run_startup().await.unwrap();
    app.run_startup().await.unwrap();
    assert_eq!(n.load(Ordering::SeqCst), 2);
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

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        app.bind(Bind::Listener(listener))
            .shutdown(async move {
                let _ = rx.await;
            })
            .serve()
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let _ = tx.send(());
    let res = tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .expect("server should stop")
        .expect("join");
    assert!(res.is_ok(), "{res:?}");
    let _ = std::net::TcpListener::bind(addr).unwrap();
}

struct FlagService {
    name: &'static str,
    flag: Arc<AtomicUsize>,
}

impl sova_core::BackgroundService for FlagService {
    fn name(&self) -> &str {
        self.name
    }

    fn run(
        self: Box<Self>,
        _state: Arc<sova_core::extend::StateMap>,
        mut shutdown: sova_core::Shutdown,
    ) -> sova_core::extend::BoxFuture<()> {
        Box::pin(async move {
            self.flag.fetch_add(1, Ordering::SeqCst);
            shutdown.recv().await;
        })
    }
}

#[tokio::test]
async fn cli_mode_skips_background_services() {
    let flag = Arc::new(AtomicUsize::new(0));
    let mut app = App::new();
    app.cli_mode(true);
    app.service(FlagService {
        name: "probe",
        flag: Arc::clone(&flag),
    });

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        app.bind(Bind::Listener(listener))
            .shutdown(async move {
                let _ = rx.await;
            })
            .serve()
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    assert_eq!(
        flag.load(Ordering::SeqCst),
        0,
        "cli_mode must skip services"
    );
    let _ = tx.send(());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .expect("server stop")
        .expect("join");
}

#[tokio::test]
async fn service_in_cli_starts_background_services() {
    let flag = Arc::new(AtomicUsize::new(0));
    let mut app = App::new();
    app.cli_mode(true).service_in_cli(true);
    app.service(FlagService {
        name: "probe",
        flag: Arc::clone(&flag),
    });

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        app.bind(Bind::Listener(listener))
            .shutdown(async move {
                let _ = rx.await;
            })
            .serve()
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    assert_eq!(
        flag.load(Ordering::SeqCst),
        1,
        "service_in_cli must start services"
    );
    let _ = tx.send(());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .expect("server stop")
        .expect("join");
}
