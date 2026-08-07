#![cfg(feature = "web")]

use http::Method;
use ruvo::{App, Meta, Response};

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
