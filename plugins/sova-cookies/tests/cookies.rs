//! CookieLayer + Response::cookie.

use http::Method;
use sova_cookies::{CookieBuilder, CookieLayer, Cookies, ResponseCookieExt};
use sova_core::{App, Request, Response};

#[tokio::test]
async fn parses_cookie_header_into_extensions() {
    let mut app = App::new();
    app.install(CookieLayer);
    app.get("/", |req: Request| async move {
        let name = req
            .get::<Cookies>()
            .and_then(|c| c.get("user").map(|s| s.to_string()))
            .unwrap_or_default();
        Response::text(name)
    });

    let req = Request::builder()
        .method(Method::GET)
        .path("/")
        .header("cookie", "user=ada; theme=dark")
        .build();
    let res = app.handle(req).await;
    assert_eq!(res.body_bytes(), Some(b"ada".as_slice()));
}

#[tokio::test]
async fn cookie_layer_as_plugin_install() {
    let mut app = App::new();
    app.install(CookieLayer);
    assert!(app.has_plugin("cookies"));
    app.get("/", |req: Request| async move {
        let theme = req
            .get::<Cookies>()
            .and_then(|c| c.get("theme").map(|s| s.to_string()))
            .unwrap_or_default();
        Response::text(theme)
    });
    let req = Request::builder()
        .method(Method::GET)
        .path("/")
        .header("cookie", "theme=dark")
        .build();
    let res = app.handle(req).await;
    assert_eq!(res.body_bytes(), Some(b"dark".as_slice()));
}

#[tokio::test]
async fn response_cookie_sets_set_cookie() {
    let mut app = App::new();
    app.get("/", |_r: Request| async {
        Response::text("ok").cookie(CookieBuilder::build(("sid", "abc")).path("/").build())
    });
    let res = app.handle_request(Method::GET, "/", "").await;
    let found = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .any(|v| v.contains("sid=abc"));
    assert!(found, "missing Set-Cookie, headers={:?}", res.headers());
}

#[tokio::test]
async fn response_cookie_overwrite_appends_set_cookie() {
    let mut app = App::new();
    app.get("/", |_r: Request| async {
        Response::text("ok")
            .cookie(CookieBuilder::build(("sid", "old")).path("/").build())
            .cookie(CookieBuilder::build(("sid", "new")).path("/").build())
    });
    let res = app.handle_request(Method::GET, "/", "").await;
    let cookies: Vec<_> = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter(|v| v.starts_with("sid="))
        .collect();
    assert_eq!(cookies.len(), 2, "headers={:?}", res.headers());
    assert!(cookies.iter().any(|c| c.contains("sid=old")));
    assert!(cookies.iter().any(|c| c.contains("sid=new")));
}

#[tokio::test]
async fn response_cookie_clear_via_max_age_zero() {
    let mut app = App::new();
    app.get("/", |_r: Request| async {
        Response::text("ok").cookie(
            CookieBuilder::build(("sid", ""))
                .path("/")
                .max_age(cookie::time::Duration::seconds(0))
                .build(),
        )
    });
    let res = app.handle_request(Method::GET, "/", "").await;
    let raw = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|c| c.starts_with("sid="))
        .expect("clear Set-Cookie");
    assert!(
        raw.contains("Max-Age=0") || raw.to_ascii_lowercase().contains("max-age=0"),
        "expected Max-Age=0, got {raw}"
    );
}
