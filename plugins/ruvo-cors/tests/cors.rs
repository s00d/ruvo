//! CORS plugin tests.

use http::Method;
use ruvo_core::{App, Plugin, Request, Response};
use ruvo_cors::Cors;

#[tokio::test]
async fn cors_preflight_has_acao() {
    let mut app = App::new();
    Cors::new().origin("*").install(&mut app);
    app.get("/api", |_r: Request| async { Response::text("ok") });

    let req = Request::builder()
        .method(Method::OPTIONS)
        .path("/api")
        .header("origin", "https://example.com")
        .header("access-control-request-method", "POST")
        .build();
    let res = app.handle(req).await;
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("*")
    );
}

#[tokio::test]
async fn origins_list_mirrors_and_varies() {
    let mut app = App::new();
    Cors::new()
        .origins(["https://a.test", "https://b.test"])
        .exposed("X-Total-Count")
        .install(&mut app);
    app.get("/", |_r: Request| async { Response::text("ok") });

    let ok = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("origin", "https://a.test")
                .build(),
        )
        .await;
    assert_eq!(
        ok.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://a.test")
    );
    assert!(ok
        .headers()
        .get("vary")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .contains("Origin"));
    assert_eq!(
        ok.headers()
            .get("access-control-expose-headers")
            .and_then(|v| v.to_str().ok()),
        Some("X-Total-Count")
    );

    let denied = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("origin", "https://evil.test")
                .build(),
        )
        .await;
    assert!(denied.headers().get("access-control-allow-origin").is_none());
}

#[tokio::test]
async fn empty_headers_reflects_acrh() {
    let mut app = App::new();
    Cors::new().headers("").install(&mut app);
    app.get("/", |_r: Request| async { Response::text("ok") });

    let res = app
        .handle(
            Request::builder()
                .method(Method::OPTIONS)
                .path("/")
                .header("origin", "https://a.test")
                .header("access-control-request-headers", "X-Custom, X-Foo")
                .build(),
        )
        .await;
    assert_eq!(
        res.headers()
            .get("access-control-allow-headers")
            .and_then(|v| v.to_str().ok()),
        Some("X-Custom, X-Foo")
    );
}

#[tokio::test]
async fn credentials_with_star_mirrors_origin() {
    let mut app = App::new();
    Cors::new().origin("*").credentials(true).install(&mut app);
    app.get("/", |_r: Request| async { Response::text("ok") });

    let res = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("origin", "https://spa.test")
                .build(),
        )
        .await;
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://spa.test")
    );
    assert_eq!(
        res.headers()
            .get("access-control-allow-credentials")
            .and_then(|v| v.to_str().ok()),
        Some("true")
    );
    assert!(res
        .headers()
        .get("vary")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .contains("Origin"));
}

#[tokio::test]
async fn default_allow_headers_include_xsrf() {
    let mut app = App::new();
    Cors::new().install(&mut app);
    app.get("/", |_r: Request| async { Response::text("ok") });

    let res = app
        .handle(
            Request::builder()
                .method(Method::OPTIONS)
                .path("/")
                .header("origin", "https://a.test")
                .header("access-control-request-method", "POST")
                .build(),
        )
        .await;
    let h = res
        .headers()
        .get("access-control-allow-headers")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(h.contains("X-CSRF-Token"), "{h}");
    assert!(h.contains("X-XSRF-TOKEN"), "{h}");
}

#[tokio::test]
async fn denied_origin_preflight_has_no_acao() {
    let mut app = App::new();
    Cors::new()
        .origins(["https://a.test"])
        .install(&mut app);
    app.get("/", |_r: Request| async { Response::text("ok") });

    let res = app
        .handle(
            Request::builder()
                .method(Method::OPTIONS)
                .path("/")
                .header("origin", "https://evil.test")
                .header("access-control-request-method", "POST")
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 204);
    assert!(res.headers().get("access-control-allow-origin").is_none());
}
