//! CookieLayer + Response::cookie.

use http::Method;
use ruvo_cookies::{CookieBuilder, CookieLayer, Cookies, ResponseCookieExt};
use ruvo_core::{App, Plugin, Request, Response};

#[tokio::test]
async fn parses_cookie_header_into_extensions() {
    let mut app = App::new();
    CookieLayer.install(&mut app);
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
