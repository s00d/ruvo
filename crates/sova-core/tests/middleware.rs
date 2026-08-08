use http::Method;
use sova_core::extend::IntoMiddleware;
use sova_core::{App, Next, Request, Response, Router};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn extensions_flow_through_middleware() {
    #[derive(Clone)]
    struct User(String);

    let mut app = App::new();
    app.use_middleware(
        (|mut req: Request, next: Next| async move {
            req.set(User("ada".into()));
            next(req).await
        })
        .into_middleware(),
    );
    app.get("/", |req: Request| async move {
        let name = req.get::<User>().map(|u| u.0.clone()).unwrap_or_default();
        Response::text(name)
    });
    assert_eq!(
        app.handle_request(Method::GET, "/", "").await.body_bytes(),
        Some(b"ada".as_slice())
    );
}

#[tokio::test]
async fn nested_mount_middleware_order() {
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));
    let counter = Arc::new(AtomicUsize::new(0));

    let mut child = Router::new();
    let log_c = Arc::clone(&log);
    let counter_c = Arc::clone(&counter);
    child.use_middleware(
        (move |req: Request, next: Next| {
            let log_c = Arc::clone(&log_c);
            let counter_c = Arc::clone(&counter_c);
            async move {
                log_c.lock().unwrap().push(format!(
                    "child-{}",
                    counter_c.fetch_add(1, Ordering::SeqCst)
                ));
                next(req).await
            }
        })
        .into_middleware(),
    );
    child.get("/", |_r: Request| async { Response::text("ok") });

    let mut app = App::new();
    let log_p = Arc::clone(&log);
    let counter_p = Arc::clone(&counter);
    app.use_middleware(
        (move |req: Request, next: Next| {
            let log_p = Arc::clone(&log_p);
            let counter_p = Arc::clone(&counter_p);
            async move {
                log_p.lock().unwrap().push(format!(
                    "parent-{}",
                    counter_p.fetch_add(1, Ordering::SeqCst)
                ));
                next(req).await
            }
        })
        .into_middleware(),
    );
    app.mount("/api", child);

    let res = app.handle_request(Method::GET, "/api", "").await;
    assert_eq!(res.body_bytes(), Some(b"ok".as_slice()));
    assert_eq!(
        *log.lock().unwrap(),
        vec!["parent-0".to_string(), "child-1".to_string()]
    );
}

#[tokio::test]
async fn child_middleware_isolated_from_sibling_mount() {
    let flag = Arc::new(AtomicUsize::new(0));

    let mut a = Router::new();
    let flag_a = Arc::clone(&flag);
    a.use_middleware(
        (move |req: Request, next: Next| {
            let flag_a = Arc::clone(&flag_a);
            async move {
                flag_a.fetch_add(1, Ordering::SeqCst);
                next(req).await
            }
        })
        .into_middleware(),
    );
    a.get("/", |_r: Request| async { Response::text("a") });

    let mut b = Router::new();
    b.get("/", |_r: Request| async { Response::text("b") });

    let mut app = App::new();
    app.mount("/a", a);
    app.mount("/b", b);

    assert_eq!(
        app.handle_request(Method::GET, "/b", "").await.body_bytes(),
        Some(b"b".as_slice())
    );
    assert_eq!(flag.load(Ordering::SeqCst), 0);
    assert_eq!(
        app.handle_request(Method::GET, "/a", "").await.body_bytes(),
        Some(b"a".as_slice())
    );
    assert_eq!(flag.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn group_middleware_not_doubled_with_root() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_r = Arc::clone(&hits);
    let hits_g = Arc::clone(&hits);

    let mut app = App::new();
    app.use_middleware(move |req: Request, next: Next| {
        let hits = Arc::clone(&hits_r);
        async move {
            hits.fetch_add(1, Ordering::SeqCst);
            next(req).await
        }
    });
    app.group("/api", |r| {
        r.use_middleware(move |req: Request, next: Next| {
            let hits = Arc::clone(&hits_g);
            async move {
                hits.fetch_add(10, Ordering::SeqCst);
                next(req).await
            }
        });
        r.get("/x", |_r: Request| async { Response::text("ok") });
    });

    let res = app.handle_request(Method::GET, "/api/x", "").await;
    assert_eq!(res.body_bytes().unwrap(), b"ok");
    assert_eq!(hits.load(Ordering::SeqCst), 11);
}
