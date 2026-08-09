use http::Method;
use sova_core::extend::{named, RouteEntry};
use sova_core::{App, Next, Request, Response, Router};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn matches_exact_and_params() {
    let mut app = App::new();
    app.get("/hello", |_req: Request| async { Response::text("hi") });
    app.get("/users/:id", |req: Request| async move {
        Response::text(req.param("id").unwrap_or("?").to_string())
    });

    assert_eq!(
        app.handle_request(Method::GET, "/hello", "")
            .await
            .body_bytes()
            .unwrap(),
        b"hi"
    );
    assert_eq!(
        app.handle_request(Method::GET, "/users/42", "")
            .await
            .body_bytes()
            .unwrap(),
        b"42"
    );
    assert_eq!(
        app.handle_request(Method::GET, "/missing", "")
            .await
            .status_code()
            .as_u16(),
        404
    );
}

#[tokio::test]
async fn mount_prefixes_and_static_priority() {
    let mut blog = Router::new();
    blog.get("/", |_r: Request| async { Response::text("blog") });
    blog.get("/new", |_r: Request| async { Response::text("new") });
    blog.get("/:slug", |req: Request| async move {
        Response::text(format!("slug:{}", req.param("slug").unwrap_or("")))
    });

    let mut app = App::new();
    app.mount("/blog", blog);

    assert_eq!(
        app.handle_request(Method::GET, "/blog", "")
            .await
            .body_bytes()
            .unwrap(),
        b"blog"
    );
    assert_eq!(
        app.handle_request(Method::GET, "/blog/new", "")
            .await
            .body_bytes()
            .unwrap(),
        b"new"
    );
    assert_eq!(
        app.handle_request(Method::GET, "/blog/hello", "")
            .await
            .body_bytes()
            .unwrap(),
        b"slug:hello"
    );
}

#[tokio::test]
async fn method_not_allowed_and_head() {
    let mut app = App::new();
    app.get("/x", |_r: Request| async { Response::text("secret") });

    let res = app.handle_request(Method::POST, "/x", "").await;
    assert_eq!(res.status_code().as_u16(), 405);
    assert!(res.headers().get(http::header::ALLOW).is_some());

    let head = app.handle_request(Method::HEAD, "/x", "").await;
    assert_eq!(head.status_code().as_u16(), 200);
    assert_eq!(head.body_bytes(), Some(b"".as_slice()));
    assert_eq!(
        head.headers()
            .get(http::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok()),
        Some("6") // "secret"
    );
}

#[tokio::test]
async fn percent_decodes_params() {
    let mut app = App::new();
    app.get("/u/:name", |req: Request| async move {
        Response::text(req.param("name").unwrap_or("").to_string())
    });
    let res = app
        .handle_request(Method::GET, "/u/John%20Doe", "")
        .await;
    assert_eq!(res.body_bytes().unwrap(), b"John Doe");
}

#[tokio::test]
async fn root_middleware_sees_404_405_options() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = Arc::clone(&hits);

    let mut app = App::new();
    app.use_middleware(move |req: Request, next: Next| {
        let hits = Arc::clone(&hits2);
        async move {
            hits.fetch_add(1, Ordering::SeqCst);
            next(req).await
        }
    });
    app.get("/ok", |_r: Request| async { Response::text("ok") });

    let _ = app.handle_request(Method::GET, "/missing", "").await;
    let _ = app.handle_request(Method::POST, "/ok", "").await;
    let _ = app.handle_request(Method::OPTIONS, "/ok", "").await;
    assert_eq!(hits.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn explain_and_route_entries() {
    let mut child = Router::new();
    child.get("/:slug", |_r: Request| async { Response::text("x") });
    let mut app = App::new();
    app.use_middleware(named("auth", |req: Request, next: Next| async move {
        next(req).await
    }));
    app.mount("/blog", child);
    app.get("/x", |_r: Request| async { Response::text("ok") });

    let text = app.explain();
    assert!(text.contains("GET /x"));
    assert!(text.contains("root_middleware: [auth]"));

    let routes = app.route_entries();
    assert!(routes.iter().any(|e| matches!(
        e,
        RouteEntry::Http { method, path, .. }
            if *method == Method::GET && path == "/blog/:slug"
    )));
}

#[tokio::test]
async fn app_handle_injects_state() {
    let mut app = App::new();
    app.state(42u32);
    app.get("/n", |req: Request| async move {
        Response::text(req.state::<u32>().to_string())
    });
    assert_eq!(
        app.handle_request(Method::GET, "/n", "")
            .await
            .body_bytes(),
        Some(b"42".as_slice())
    );
}
