//! Integration tests for sova-i18n.

use http::Method;
use sova_core::{App, Request, Response};
use sova_i18n::{default_plural, I18n, I18nExt, I18nRouteExt, Locale};
use sova_openapi::{Doc, OpenApiDocExt};

fn write_locales(dir: &std::path::Path) {
    std::fs::write(
        dir.join("en.json"),
        r#"{
            "nav": { "about": "About" },
            "greet": "Hello {name}",
            "cart.items": "none|one|many",
            "only_en": "EN"
        }"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("de.json"),
        r#"{
            "nav": { "about": "Über" },
            "greet": "Hallo {name}",
            "cart.items": "keine|eins|viele"
        }"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("pages/blog")).unwrap();
    std::fs::write(
        dir.join("pages/blog/en.json"),
        r#"{"title":"Post","nav":{"about":"Blog About"}}"#,
    )
    .unwrap();
}

#[tokio::test]
async fn translations_and_fallback_and_plural() {
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en"), Locale::new("de")])
            .fallback("en")
            .path_prefix(false),
    );

    app.get("/t", |req: Request| async move {
        Response::text(format!(
            "{}|{}|{}|{}",
            req.t("nav.about"),
            req.t_args("greet", &[("name", "Ada")]),
            req.tn("cart.items", 0),
            req.t("only_en")
        ))
    });

    let en = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/t")
                .header("accept-language", "en")
                .build(),
        )
        .await;
    assert_eq!(
        en.body_bytes(),
        Some(b"About|Hello Ada|none|EN".as_slice())
    );
    assert_eq!(
        en.headers()
            .get("content-language")
            .and_then(|v| v.to_str().ok()),
        Some("en")
    );
    assert!(en.headers().get("vary").is_some());

    let de = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/t")
                .header("accept-language", "de")
                .build(),
        )
        .await;
    assert_eq!(
        de.body_bytes(),
        Some("Über|Hallo Ada|keine|EN".as_bytes())
    );
}

#[test]
fn plural_boundaries() {
    assert_eq!(
        default_plural("k", 0, "en", &["none", "one", "many"]),
        "none"
    );
    assert_eq!(
        default_plural("k", 1, "en", &["none", "one", "many"]),
        "one"
    );
    assert_eq!(
        default_plural("k", 5, "en", &["none", "one", "many"]),
        "many"
    );
    assert_eq!(
        default_plural("k", 99, "en", &["none", "one", "many"]),
        "many"
    );
}

#[tokio::test]
async fn page_scope_contains_root_and_override() {
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en"), Locale::new("de")])
            .path_prefix(false),
    );

    app.get("/blog", |req: Request| async move {
        Response::text(format!("{}|{}", req.t("title"), req.t("nav.about")))
    })
    .i18n_scope("blog");

    let res = app.handle_request(Method::GET, "/blog", "").await;
    assert_eq!(res.body_bytes(), Some(b"Post|Blog About".as_slice()));
}

#[tokio::test]
async fn missing_key_returns_key() {
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en")])
            .path_prefix(false),
    );
    app.get("/m", |req: Request| async move { Response::text(req.t("nope")) });
    let res = app.handle_request(Method::GET, "/m", "").await;
    assert_eq!(res.body_bytes(), Some(b"nope".as_slice()));

    let miss = app
        .handle_request(Method::GET, "/_i18n/_missing.json", "")
        .await;
    let body = String::from_utf8_lossy(miss.body_bytes().unwrap());
    assert!(body.contains("nope"));
}

#[tokio::test]
async fn etag_304_and_version_cache() {
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en")])
            .path_prefix(false),
    );

    let first = app
        .handle_request(Method::GET, "/_i18n/en.json", "")
        .await;
    assert_eq!(first.status_code().as_u16(), 200);
    let etag = first
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let second = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/_i18n/en.json")
                .header("if-none-match", &etag)
                .build(),
        )
        .await;
    assert_eq!(second.status_code().as_u16(), 304);

    let locales = app
        .handle_request(Method::GET, "/_i18n/locales.json", "")
        .await;
    let v: serde_json::Value =
        serde_json::from_slice(locales.body_bytes().unwrap()).unwrap();
    let version = v["version"].as_str().unwrap();
    let cached = app
        .handle_request(Method::GET, &format!("/_i18n/en.json?v={version}"), "")
        .await;
    assert!(cached
        .headers()
        .get("cache-control")
        .and_then(|h| h.to_str().ok())
        .unwrap()
        .contains("immutable"));
}

#[tokio::test]
async fn resolve_url_beats_query() {
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en"), Locale::new("de")])
            .path_prefix(true),
    );
    app.get("/de/x", |req: Request| async move {
        Response::text(req.locale().to_string())
    });
    let mut req = Request::builder().method(Method::GET).path("/de/x").build();
    req.query.insert("locale".into(), "en".into());
    let res = app.handle(req).await;
    assert_eq!(res.body_bytes(), Some(b"de".as_slice()));
}

#[tokio::test]
async fn doc_and_i18n_scope_coexist() {
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en")])
            .path_prefix(false),
    );

    app.get("/blog/:slug", |req: Request| async move {
        Response::text(req.t("title"))
    })
    .doc(Doc::new().ok_schema(serde_json::json!({ "type": "string" })))
    .i18n_scope("blog");

    let entries = app.route_entries();
    let meta = match entries
        .iter()
        .find(|e| {
            matches!(
                e,
                sova_core::extend::RouteEntry::Http { path, .. } if path == "/blog/:slug"
            )
        })
        .unwrap()
    {
        sova_core::extend::RouteEntry::Http { meta, .. } => meta,
        _ => panic!(),
    };
    assert!(meta.get::<Doc>().is_some());
    assert!(meta.get::<sova_i18n::I18nScope>().is_some());

    let res = app.handle_request(Method::GET, "/blog/hi", "").await;
    assert_eq!(res.body_bytes(), Some(b"Post".as_slice()));
}

#[tokio::test]
#[cfg(feature = "cookie")]
async fn cookie_without_layer_fails_requires() {
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en")])
            .path_prefix(false)
            .cookie("locale"),
    );
    assert!(app.run_startup().await.is_err());
}

#[tokio::test]
#[cfg(feature = "cookie")]
async fn cookie_with_layer_ok() {
    use sova_cookies::CookieLayer;
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(CookieLayer);
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en"), Locale::new("de")])
            .path_prefix(false)
            .cookie("locale"),
    );
    app.run_startup().await.unwrap();
    app.get("/x", |req: Request| async move { Response::text(req.locale().to_string()) });
    let res = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/x")
                .header("cookie", "locale=de")
                .build(),
        )
        .await;
    assert_eq!(res.body_bytes(), Some(b"de".as_slice()));
}
