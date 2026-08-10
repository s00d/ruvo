//! Integration tests for the Static plugin.

use http::Method;
use sova_core::extend::IntoMiddleware;
use sova_core::{App, Next, Request, Response, Router};
use sova_static::Static;

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

    let res = app.handle_request(Method::GET, "/files/link.txt", "").await;
    assert_eq!(res.status_code().as_u16(), 403);
}

#[tokio::test]
async fn static_under_module_guard_is_401() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("secret.txt"), b"nope").unwrap();

    let mut admin = Router::new();
    admin.use_middleware(
        (|_req: Request, _next: Next| async move { Response::text("Unauthorized").status(401) })
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

#[tokio::test]
async fn static_denies_dotfiles_by_default() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".env"), b"secret").unwrap();

    let mut app = App::new();
    app.install(Static::new("/f", dir.path()));
    let res = app.handle_request(Method::GET, "/f/.env", "").await;
    assert_eq!(res.status_code().as_u16(), 403);

    let mut app2 = App::new();
    app2.install(Static::new("/f", dir.path()).dotfiles_allow());
    let ok = app2.handle_request(Method::GET, "/f/.env", "").await;
    assert_eq!(ok.status_code().as_u16(), 200);
}

#[tokio::test]
async fn static_max_age_immutable() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"hi").unwrap();
    let mut app = App::new();
    app.install(
        Static::new("/f", dir.path())
            .max_age(std::time::Duration::from_secs(9))
            .immutable(true),
    );
    let res = app.handle_request(Method::GET, "/f/a.txt", "").await;
    assert_eq!(
        res.headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("public, max-age=9, immutable")
    );
}

#[tokio::test]
async fn static_max_age_from_toml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"hi").unwrap();
    let mut app = App::new();
    app.configure_from_str(
        r#"
[static]
max_age = "42s"
"#,
    )
    .unwrap();
    app.install(Static::new("/f", dir.path()));
    let res = app.handle_request(Method::GET, "/f/a.txt", "").await;
    assert_eq!(res.status_code().as_u16(), 200);
    assert_eq!(
        res.headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("public, max-age=42")
    );
}

#[tokio::test]
async fn static_serves_nested_file() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("css");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("app.css"), b"body{}").unwrap();

    let mut app = App::new();
    app.install(Static::new("/assets", dir.path()));
    let mut res = app
        .handle_request(Method::GET, "/assets/css/app.css", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    let ct = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.contains("css"), "content-type={ct}");
    let body = res.take_body().collect().await.unwrap();
    assert_eq!(&body[..], b"body{}");
}

#[tokio::test]
async fn static_range_and_if_modified_since() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"0123456789").unwrap();

    let mut app = App::new();
    app.install(Static::new("/f", dir.path()));

    let first = app.handle_request(Method::GET, "/f/a.txt", "").await;
    assert_eq!(first.status_code().as_u16(), 200);
    let lm = first
        .headers()
        .get("last-modified")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let ims = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/f/a.txt")
                .header("if-modified-since", &lm)
                .build(),
        )
        .await;
    assert_eq!(ims.status_code().as_u16(), 304);

    let partial = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/f/a.txt")
                .header("range", "bytes=0-3")
                .build(),
        )
        .await;
    assert_eq!(partial.status_code().as_u16(), 206);
    assert_eq!(
        partial
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok()),
        Some("bytes 0-3/10")
    );
    assert_eq!(partial.body_bytes().unwrap(), b"0123");

    let unsat = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/f/a.txt")
                .header("range", "bytes=100-200")
                .build(),
        )
        .await;
    assert_eq!(unsat.status_code().as_u16(), 416);

    let bad_path = app
        .handle_request(Method::GET, "/f/../etc/passwd", "")
        .await;
    assert_eq!(bad_path.status_code().as_u16(), 403);
}

#[tokio::test]
async fn static_index_html_and_directory_forbidden() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), b"<h1>hi</h1>").unwrap();
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();

    let mut app = App::new();
    app.install(Static::new("/site", dir.path()).index(true));
    let mut idx = app.handle_request(Method::GET, "/site", "").await;
    assert_eq!(idx.status_code().as_u16(), 200);
    let body = idx.take_body().collect().await.unwrap();
    assert_eq!(&body[..], b"<h1>hi</h1>");

    let dir_res = app.handle_request(Method::GET, "/site/sub", "").await;
    assert_eq!(dir_res.status_code().as_u16(), 403);
}

#[tokio::test]
async fn static_missing_file_404() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new();
    app.install(Static::new("/f", dir.path()));
    let res = app.handle_request(Method::GET, "/f/nope.txt", "").await;
    assert_eq!(res.status_code().as_u16(), 404);
}
