use http::Method;
use serde_json::json;
use sova_core::{App, Json, Request, Response};
use sova_meta::{
    absolute_url, render_html, resolve_meta, strip_tracking, Article, Meta, MetaExt, Robots,
    Sitemap, TrailingSlash,
};

#[tokio::test]
async fn field_merge_and_html() {
    let mut app = App::new();
    app.install(
        Meta::new()
            .site_name("Shop")
            .title_template("{} — Shop")
            .public_url("https://shop.example.com"),
    );
    app.get("/about", |mut req: Request| async move {
        req.meta().description("About us");
        let html = render_html(&req.resolved_meta());
        Response::html(html)
    })
    .with(Meta::page().title("About"));

    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/about", "").await;
    let body = String::from_utf8(res.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("<title>About — Shop</title>"));
    assert!(body.contains("About us"));
    assert!(body.contains("rel=\"canonical\""));
    assert!(body.contains("https://shop.example.com/about"));
}

#[tokio::test]
async fn strips_utm_from_canonical_path_helper() {
    assert_eq!(strip_tracking("/p", "utm_source=x&id=1"), "/p?id=1");
    assert_eq!(absolute_url("https://ex.com", "/a"), "https://ex.com/a");
}

#[tokio::test]
async fn slash_redirect() {
    let mut app = App::new();
    app.install(
        Meta::new()
            .public_url("https://ex.com")
            .trailing_slash(TrailingSlash::Never),
    );
    app.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));

    let server = app.build().unwrap();
    let res = server
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/about/")
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 301);
    assert_eq!(
        res.headers().get("location").and_then(|v| v.to_str().ok()),
        Some("/about")
    );
}

#[tokio::test]
async fn handler_noindex_sets_x_robots_tag_on_json() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.get("/api/private", |mut req: Request| async move {
        req.meta().noindex();
        Json(json!({ "ok": true }))
    });

    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/api/private", "").await;
    assert_eq!(
        res.headers()
            .get("x-robots-tag")
            .and_then(|v| v.to_str().ok()),
        Some("noindex")
    );
}

#[tokio::test]
async fn injects_head_into_bare_html() {
    use sova_core::Html;

    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com").site_name("S"));
    app.get("/about", || async { Html("<h1>About</h1>".to_string()) })
        .with(Meta::page().title("About").description("About page"));

    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/about", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("<title>About</title>"), "{body}");
    assert!(body.contains("About page"), "{body}");
    assert!(body.contains("<h1>About</h1>"), "{body}");
}

#[tokio::test]
async fn manual_skips_inject() {
    use sova_core::Html;

    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.get("/raw", || async { Html("<h1>Raw</h1>".to_string()) })
        .with(Meta::page().title("T").description("d").manual());

    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/raw", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert_eq!(body, "<h1>Raw</h1>");
}

#[tokio::test]
async fn moved_to_redirects_301() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.get("/old", || async { Response::text("should not see") })
        .with(Meta::page().moved_to("/new").title("x").description("y"));

    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/old", "").await;
    assert_eq!(res.status_code().as_u16(), 301);
    assert_eq!(
        res.headers().get("location").and_then(|v| v.to_str().ok()),
        Some("/new")
    );
}

#[tokio::test]
async fn sitemap_excludes_noindex_and_doc() {
    use sova_openapi::{Doc, OpenApiDocExt};

    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(Sitemap::new());
    app.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));
    app.get("/secret", |_r: Request| async { Response::text("ok") })
        .with(Meta::noindex());
    app.get("/api/x", |_r: Request| async { Response::text("ok") })
        .doc(Doc::new().ok_schema(json!({ "type": "object" })));

    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/sitemap.xml", "").await;
    let body = String::from_utf8(res.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("/about"));
    assert!(!body.contains("/secret"));
    assert!(!body.contains("/api/x"));
}

#[tokio::test]
async fn sitemap_exclude_and_include() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(
        Sitemap::new()
            .exclude("/admin/*")
            .include("/app")
            .include("/pricing"),
    );
    app.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));
    app.get("/admin/users", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("Admin").description("d"));

    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/sitemap.xml", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("/about"));
    assert!(body.contains("/app"));
    assert!(body.contains("/pricing"));
    assert!(!body.contains("/admin"));
}

#[tokio::test]
async fn robots_block_all_from_config() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[default.meta]
robots = "block-all"
public_url = "https://ex.com"
"#,
    )
    .unwrap();
    app.install(Meta::new());
    app.install(Robots::new());
    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/robots.txt", "").await;
    let body = String::from_utf8(res.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("Disallow: /"));
}

#[tokio::test]
async fn robots_builder_disallow_and_block_all() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(Sitemap::new());
    app.install(Robots::new().disallow("/admin").disallow("/api"));
    app.get("/secret", |_r: Request| async { Response::text("ok") })
        .with(Meta::noindex().title("S").description("d"));

    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/robots.txt", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("Allow: /"));
    assert!(body.contains("Disallow: /admin"));
    assert!(body.contains("Disallow: /api"));
    assert!(body.contains("Disallow: /secret"));
    assert!(body.contains("Sitemap: https://ex.com/sitemap.xml"));

    let mut app2 = App::new();
    app2.install(Robots::new().block_all());
    let body2 = String::from_utf8(
        app2.build()
            .unwrap()
            .handle_request(Method::GET, "/robots.txt", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert_eq!(body2.trim(), "User-agent: *\nDisallow: /");
}

#[tokio::test]
async fn robots_allow_before_sitemap() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(Sitemap::new());
    app.install(Robots::new());
    app.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));
    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/robots.txt", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let allow = body.find("Allow: /").expect("Allow");
    let sitemap = body.find("Sitemap:").expect("Sitemap");
    assert!(allow < sitemap, "Allow must precede Sitemap:\n{body}");
}

#[tokio::test]
async fn jsonld_in_html() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com").site_name("S"));
    app.get("/p", |mut req: Request| async move {
        req.meta()
            .title("Post")
            .description("d")
            .jsonld_schema(&Article {
                headline: "Post".into(),
                ..Default::default()
            });
        Response::html(render_html(&resolve_meta(&req)))
    });
    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/p", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("application/ld+json"));
    assert!(body.contains("Article"));
}

#[tokio::test]
async fn sitemap_provider() {
    use sova_meta::{ChangeFreq, Entry};

    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(Sitemap::new().provider("/blog/:slug", |_ctx| async move {
        Ok(vec![
            Entry::new("/blog/one").changefreq(ChangeFreq::Weekly),
            Entry::new("/blog/two"),
        ])
    }));
    app.get("/home", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("H").description("d"));

    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/sitemap.xml", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("/blog/one"));
    assert!(body.contains("/blog/two"));
    assert!(body.contains("/home"));
}

#[tokio::test]
async fn sitemap_provider_error_is_500() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(Sitemap::new().provider("/broken", |_ctx| async move { Err("db down".into()) }));
    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/sitemap.xml", "").await;
    assert_eq!(res.status_code().as_u16(), 500);
}

#[cfg(feature = "store")]
#[tokio::test]
async fn sitemap_kv_cache_avoids_second_provider_call() {
    use sova_store::{namespace, MemoryStore};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = Arc::clone(&hits);
    let store = Arc::new(namespace(Arc::new(MemoryStore::new()), "meta"));

    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(
        Sitemap::new()
            .ttl(Duration::from_secs(60))
            .cache_store(store)
            .provider("/blog/:slug", move |_ctx| {
                let hits = Arc::clone(&hits2);
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    Ok(vec![sova_meta::Entry::new("/blog/one")])
                }
            }),
    );
    let server = app.build().unwrap();
    let _ = server.handle_request(Method::GET, "/sitemap.xml", "").await;
    let _ = server.handle_request(Method::GET, "/sitemap.xml", "").await;
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "i18n")]
#[tokio::test]
async fn x_default_matches_first_seo_locale() {
    use sova_i18n::{I18n, Locale};

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("en.json"), r#"{"a":"1"}"#).unwrap();
    std::fs::write(dir.path().join("de.json"), r#"{"a":"1"}"#).unwrap();

    let mut app = App::new();
    app.install(
        I18n::new(
            dir.path(),
            vec![
                Locale::new("en").with_seo(false),
                Locale::new("de").with_iso("de-DE").with_seo(true),
            ],
        )
        .default_locale("en")
        .path_prefix(true),
    );
    app.install(Meta::new().public_url("https://ex.com").site_name("S"));
    app.get("/blog", |mut req: Request| async move {
        req.meta().title("Blog").description("d");
        Response::html(render_html(&req.resolved_meta()))
    })
    .with(Meta::page().title("Blog").description("d"));

    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/blog", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();

    assert!(body.contains("hreflang=\"de-DE\""));
    assert!(body.contains("href=\"https://ex.com/blog\""));
    assert!(body.contains("hreflang=\"x-default\""));
    assert!(
        !body.contains("hreflang=\"en\""),
        "non-seo locale must be omitted:\n{body}"
    );
    let xd = body
        .split("hreflang=\"x-default\"")
        .nth(1)
        .and_then(|s| s.split("href=\"").nth(1))
        .and_then(|s| s.split('"').next())
        .unwrap();
    assert_eq!(xd, "https://ex.com/blog");
}

#[cfg(feature = "i18n")]
#[tokio::test]
async fn sitemap_includes_xhtml_alternates() {
    use sova_i18n::{I18n, Locale};

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("en.json"), r#"{"a":"1"}"#).unwrap();
    std::fs::write(dir.path().join("de.json"), r#"{"a":"1"}"#).unwrap();

    let mut app = App::new();
    app.install(
        I18n::new(
            dir.path(),
            vec![Locale::new("en"), Locale::new("de").with_iso("de")],
        )
        .default_locale("en")
        .path_prefix(true),
    );
    app.install(Meta::new().public_url("https://ex.com"));
    app.install(Sitemap::new());
    app.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));

    let server = app.build().unwrap();
    let body = String::from_utf8(
        server
            .handle_request(Method::GET, "/sitemap.xml", "")
            .await
            .body_bytes()
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("xmlns:xhtml="));
    assert!(body.contains("xhtml:link"));
    assert!(body.contains("hreflang=\"x-default\""));
    assert!(body.contains("/de/about"));
}
