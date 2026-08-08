//! CSRF + session integration (mirrors unit patterns in src).

use http::Method;
use sova_core::{App, Request, Response};
use sova_csrf::{Csrf, CsrfExt};
use sova_session::memory_sessions;

fn app_with_csrf() -> App {
    let mut app = App::new();
    app.install(memory_sessions());
    app.install(Csrf::new());
    app.get("/", |req: Request| async move { Response::text(req.csrf_token()) });
    app.post("/echo", |_req: Request| async { Response::text("ok") });
    app
}

fn session_cookie(res: &Response) -> String {
    res.headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|c| c.starts_with("sova_sid="))
        .map(|c| c.split(';').next().unwrap().to_string())
        .expect("session cookie")
}

fn cookie_named(res: &Response, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    res.headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|c| c.starts_with(&prefix))
        .map(|c| {
            c.split(';')
                .next()
                .unwrap()
                .trim_start_matches(&prefix)
                .to_string()
        })
}

#[tokio::test]
async fn get_returns_csrf_token() {
    let app = app_with_csrf();
    let res = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    let body = String::from_utf8_lossy(res.body_bytes().unwrap()).into_owned();
    assert!(!body.is_empty());
    assert!(session_cookie(&res).starts_with("sova_sid="));
}

#[tokio::test]
async fn get_sets_xsrf_cookie() {
    let app = app_with_csrf();
    let res = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    let body = String::from_utf8_lossy(res.body_bytes().unwrap()).into_owned();
    assert_eq!(cookie_named(&res, "XSRF-TOKEN").as_deref(), Some(body.as_str()));
}

#[tokio::test]
async fn xsrf_cookie_can_be_disabled() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.install(Csrf::new().xsrf_cookie(false));
    app.get("/", |req: Request| async move { Response::text(req.csrf_token()) });

    let res = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    assert!(cookie_named(&res, "XSRF-TOKEN").is_none());
}

#[tokio::test]
async fn post_without_token_is_403() {
    let app = app_with_csrf();
    let get = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    let cookie = session_cookie(&get);

    let res = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/echo")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body("x=1")
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 403);
}

#[tokio::test]
async fn post_with_token_ok() {
    let app = app_with_csrf();
    let get = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    let cookie = session_cookie(&get);
    let get2 = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("cookie", &cookie)
                .build(),
        )
        .await;
    let token = String::from_utf8_lossy(get2.body_bytes().unwrap()).into_owned();

    let res = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/echo")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(format!("csrf={token}"))
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    assert_eq!(res.body_bytes(), Some(b"ok".as_slice()));
}

#[tokio::test]
async fn post_with_xsrf_header_ok() {
    let app = app_with_csrf();
    let get = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    let cookie = session_cookie(&get);
    let token = cookie_named(&get, "XSRF-TOKEN").expect("xsrf");

    let res = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/echo")
                .header("cookie", &cookie)
                .header("x-xsrf-token", &token)
                .header("content-type", "application/json")
                .body("{}")
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
}

#[tokio::test]
async fn post_mismatch_token_is_403() {
    let app = app_with_csrf();
    let get = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    let cookie = session_cookie(&get);

    let res = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/echo")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body("csrf=deadbeefdeadbeefdeadbeefdeadbeef")
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 403);
}

#[tokio::test]
async fn skip_skips_path() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.install(Csrf::new().skip("/echo"));
    app.post("/echo", |_req: Request| async { Response::text("ok") });

    let res = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/echo")
                .header("content-type", "application/json")
                .body("{}")
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
}

#[tokio::test]
async fn except_skips_path() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.install(Csrf::new().except("/hook/*"));
    app.post("/hook/a", |_req: Request| async { Response::text("ok") });
    app.post("/secure", |_req: Request| async { Response::text("ok") });

    let skipped = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/hook/a")
                .header("content-type", "application/json")
                .body("{}")
                .build(),
        )
        .await;
    assert_eq!(skipped.status_code().as_u16(), 200);

    let locked = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/secure")
                .header("content-type", "application/json")
                .body("{}")
                .build(),
        )
        .await;
    assert_eq!(locked.status_code().as_u16(), 403);
}

#[tokio::test]
async fn only_limits_paths() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.install(Csrf::new().only("/secure/*"));
    app.post("/open", |_req: Request| async { Response::text("ok") });
    app.post("/secure/x", |_req: Request| async { Response::text("ok") });

    let open = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/open")
                .header("content-type", "application/json")
                .body("{}")
                .build(),
        )
        .await;
    assert_eq!(open.status_code().as_u16(), 200);

    let locked = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/secure/x")
                .header("content-type", "application/json")
                .body("{}")
                .build(),
        )
        .await;
    assert_eq!(locked.status_code().as_u16(), 403);
}

#[tokio::test]
async fn query_token_ok() {
    let app = app_with_csrf();
    let get = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    let cookie = session_cookie(&get);
    let token = cookie_named(&get, "XSRF-TOKEN").expect("xsrf");

    let res = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path(format!("/echo?csrf={token}"))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body("{}")
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
}

#[tokio::test]
async fn multipart_deferred_without_header() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.install(Csrf::new());
    app.get("/", |req: Request| async move { Response::text(req.csrf_token()) });
    app.post("/upload", |_req: Request| async { Response::text("ok") });

    let get = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    let cookie = session_cookie(&get);

    let res = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/upload")
                .header("cookie", &cookie)
                .header("content-type", "multipart/form-data; boundary=----x")
                .body("------x--\r\n")
                .build(),
        )
        .await;
    // Deferred: middleware lets the request through without token.
    assert_eq!(res.status_code().as_u16(), 200);
}

#[tokio::test]
async fn verify_csrf_helper() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.install(Csrf::new().auto(false));
    app.get("/", |req: Request| async move { Response::text(req.csrf_token()) });
    app.post("/manual", |req: Request| async move {
        let q = req.query("csrf").map(str::to_owned);
        match req.verify_csrf(q.as_deref()) {
            Ok(()) => Response::text("ok"),
            Err(_) => Response::text("bad").status(400),
        }
    });

    let get = app
        .handle(Request::builder().method(Method::GET).path("/").build())
        .await;
    let cookie = session_cookie(&get);
    let token = String::from_utf8_lossy(get.body_bytes().unwrap()).into_owned();

    let bad = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/manual")
                .header("cookie", &cookie)
                .build(),
        )
        .await;
    assert_eq!(bad.status_code().as_u16(), 400);

    let ok = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path(format!("/manual?csrf={token}"))
                .header("cookie", &cookie)
                .build(),
        )
        .await;
    assert_eq!(ok.status_code().as_u16(), 200);
}
