#![cfg(feature = "web")]

use http::Method;
use sova::{App, Meta, Response};

#[tokio::test]
async fn web_preset_serves_sitemap_and_robots() {
    let mut app = App::web()
        .site("Blog")
        .public_url("https://ex.com")
        .into_app();
    app.get("/about", |_r| async { Response::text("ok") })
        .with(Meta::page().title("About").description("About page"));

    let server = app.build().unwrap();

    let sitemap = String::from_utf8(
        server
            .handle_request(Method::GET, "/sitemap.xml", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(sitemap.contains("/about"), "{sitemap}");

    let robots = String::from_utf8(
        server
            .handle_request(Method::GET, "/robots.txt", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(robots.contains("Allow: /"), "{robots}");
    assert!(
        robots.contains("Sitemap: https://ex.com/sitemap.xml"),
        "{robots}"
    );
}

#[tokio::test]
async fn web_preset_with_views_and_assets_dirs() {
    let root = tempfile::tempdir().unwrap();
    let views = root.path().join("views");
    let assets = root.path().join("public");
    std::fs::create_dir_all(&views).unwrap();
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(views.join("home.html"), "hello {{ name }}").unwrap();
    std::fs::write(assets.join("app.css"), "body{}").unwrap();

    let mut app = App::web()
        .site("Demo")
        .public_url("https://demo.test")
        .views(&views)
        .assets(&assets)
        .assets_mount("/static")
        .into_app();
    app.get("/", |_r| async { Response::text("home") });

    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/", "").await;
    assert_eq!(res.body_bytes(), Some(b"home".as_slice()));

    let css = server
        .handle_request(Method::GET, "/static/app.css", "")
        .await;
    assert_eq!(css.status_code().as_u16(), 200, "static asset status");
    // File responses may be streamed — presence of a successful status is enough.
    if let Some(bytes) = css.body_bytes() {
        assert_eq!(bytes, b"body{}");
    }
}
