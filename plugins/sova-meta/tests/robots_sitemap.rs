//! Robots / Sitemap plugin install paths (toml + routes).

use http::Method;
use sova_core::{App, Request, Response};
use sova_meta::{Meta, Robots, Sitemap};

#[tokio::test]
async fn robots_install_from_toml_section() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[meta]
public_url = "https://ex.com"

[robots]
path = "/robots.txt"
allow = ["/cdn"]
disallow = ["/admin", "/tmp"]
from_noindex = true
sitemap = "https://ex.com/custom-sitemap.xml"
"#,
    )
    .unwrap();
    app.install(Meta::new());
    app.install(Robots::new());
    app.get("/secret", |_r: Request| async { Response::text("ok") })
        .with(Meta::noindex().title("S").description("d"));

    let body = String::from_utf8(
        app.build()
            .unwrap()
            .handle_request(Method::GET, "/robots.txt", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("Allow: /cdn"), "{body}");
    assert!(body.contains("Disallow: /admin"), "{body}");
    assert!(body.contains("Disallow: /tmp"), "{body}");
    assert!(body.contains("Disallow: /secret"), "{body}");
    assert!(
        body.contains("Sitemap: https://ex.com/custom-sitemap.xml"),
        "{body}"
    );
}

#[tokio::test]
async fn robots_builder_user_agent_and_raw() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(
        Robots::new()
            .allow("/public")
            .crawl_delay(1.5)
            .user_agent("Googlebot", |ua| {
                ua.disallow("/nogoogle").crawl_delay(2.0).raw("# custom")
            })
            .raw("# trailing")
            .sitemap("https://ex.com/s.xml"),
    );

    let body = String::from_utf8(
        app.build()
            .unwrap()
            .handle_request(Method::GET, "/robots.txt", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("User-agent: *"), "{body}");
    assert!(body.contains("Allow: /public"), "{body}");
    assert!(body.contains("Crawl-delay: 1.5") || body.contains("1.5"), "{body}");
    assert!(body.contains("User-agent: Googlebot"), "{body}");
    assert!(body.contains("Disallow: /nogoogle"), "{body}");
    assert!(body.contains("# custom"), "{body}");
    assert!(body.contains("# trailing"), "{body}");
    assert!(body.contains("Sitemap: https://ex.com/s.xml"), "{body}");
}

#[tokio::test]
async fn robots_block_all_from_robots_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[robots]
block_all = true
"#,
    )
    .unwrap();
    app.install(Robots::new());
    let body = String::from_utf8(
        app.build()
            .unwrap()
            .handle_request(Method::GET, "/robots.txt", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("Disallow: /"), "{body}");
}

#[tokio::test]
async fn sitemap_requires_public_url() {
    let mut app = App::new();
    app.install(Meta::new());
    app.install(Sitemap::new());
    let res = app
        .build()
        .unwrap()
        .handle_request(Method::GET, "/sitemap.xml", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 500);
}

#[tokio::test]
async fn sitemap_paginated_route() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(Sitemap::new());
    app.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));

    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/sitemap-1.xml", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("/about"), "{body}");
    assert!(body.contains("urlset") || body.contains("<url>"), "{body}");
}

#[tokio::test]
async fn sitemap_from_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[meta]
public_url = "https://ex.com"

[sitemap]
path = "/sitemap.xml"
ttl = 60
exclude = ["/admin/*"]
include = ["/landing"]
"#,
    )
    .unwrap();
    app.install(Meta::new());
    app.install(Sitemap::new());
    app.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));
    app.get("/admin/x", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("Admin").description("d"));

    let body = String::from_utf8(
        app.build()
            .unwrap()
            .handle_request(Method::GET, "/sitemap.xml", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("/about"), "{body}");
    assert!(body.contains("/landing"), "{body}");
    assert!(!body.contains("/admin"), "{body}");
}
