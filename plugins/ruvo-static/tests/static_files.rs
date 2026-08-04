//! Integration tests for the Static plugin.

use http::Method;
use ruvo_core::extend::IntoMiddleware;
use ruvo_core::{App, Next, Request, Response, Router};
use ruvo_static::Static;

#[cfg(unix)]
#[tokio::test]
async fn static_blocks_symlink_escape() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.txt"), b"leak").unwrap();
    let link = dir.path().join("link.txt");
    std::os::unix::fs::symlink(outside.path().join("secret.txt"), &link).unwrap();

    let mut app = App::new();
    app.install(Static::new("/files", dir.path()));

    let res = app
        .handle_request(Method::GET, "/files/link.txt", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 403);
}

#[tokio::test]
async fn static_under_module_guard_is_401() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("secret.txt"), b"nope").unwrap();

    let mut admin = Router::new();
    admin.use_middleware(
        (|_req: Request, _next: Next| async move {
            Response::text("Unauthorized").status(401)
        })
        .into_middleware(),
    );
    Static::new("/files", dir.path()).register(&mut admin);

    let mut app = App::new();
    app.mount("/admin", admin);

    let res = app
        .handle_request(Method::GET, "/admin/files/secret.txt", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 401);
}

#[tokio::test]
async fn static_if_none_match_returns_304() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"hello-static").unwrap();

    let mut app = App::new();
    app.install(Static::new("/f", dir.path()));

    let first = app.handle_request(Method::GET, "/f/a.txt", "").await;
    assert_eq!(first.status_code().as_u16(), 200);
    let etag = first
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let req = Request::builder()
        .method(Method::GET)
        .path("/f/a.txt")
        .header("if-none-match", &etag)
        .build();
    let res = app.handle(req).await;
    assert_eq!(res.status_code().as_u16(), 304);
}
