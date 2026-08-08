//! Extra i18n coverage: handlers, all.json, cookie Set-Cookie, toml unset.

use http::Method;
use ruvo_core::{App, Request, Response};
use ruvo_i18n::{I18n, I18nExt, Locale};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn write_locales(dir: &std::path::Path) {
    std::fs::write(
        dir.join("en.json"),
        r#"{"greet":"Hello {name}","cart.items":"none|one|many"}"#,
    )
    .unwrap();
    std::fs::write(dir.join("de.json"), r#"{"greet":"Hallo {name}"}"#).unwrap();
}

#[tokio::test]
async fn plural_fn_missing_handler_and_all_json() {
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let misses = Arc::new(AtomicUsize::new(0));
    let misses2 = Arc::clone(&misses);

    let mut app = App::new();
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en"), Locale::new("de")])
            .fallback("en")
            .path_prefix(false)
            .enable_all_json(true)
            .plural_fn(|_k, n, _loc, forms| {
                if n == 0 {
                    forms[0].to_string()
                } else if n == 1 {
                    forms.get(1).unwrap_or(&forms[0]).to_string()
                } else {
                    forms.last().unwrap_or(&forms[0]).to_string()
                }
            })
            .missing_handler(move |_loc, _scope, _key| {
                misses2.fetch_add(1, Ordering::SeqCst);
            }),
    );

    app.get("/p", |req: Request| async move {
        Response::text(format!(
            "{}|{}",
            req.tn("cart.items", 2),
            req.t("missing.key")
        ))
    });

    let res = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/p")
                .header("accept-language", "en")
                .build(),
        )
        .await;
    assert_eq!(res.body_bytes(), Some(b"many|missing.key".as_slice()));
    assert!(misses.load(Ordering::SeqCst) >= 1);

    let all = app
        .handle_request(Method::GET, "/_i18n/all.json", "")
        .await;
    assert_eq!(all.status_code().as_u16(), 200);
    let body = String::from_utf8_lossy(all.body_bytes().unwrap());
    assert!(body.contains("greet") || body.contains("Hello"));
}

#[tokio::test]
#[cfg(feature = "cookie")]
async fn set_locale_cookie_on_response() {
    use ruvo_cookies::CookieLayer;
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(CookieLayer);
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en"), Locale::new("de")])
            .path_prefix(false)
            .cookie("locale")
            .set_locale_cookie(true),
    );
    app.get("/x", |req: Request| async move { Response::text(req.locale().to_string()) });
    app.run_startup().await.unwrap();

    let res = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/x")
                .header("accept-language", "de")
                .build(),
        )
        .await;
    assert_eq!(res.body_bytes(), Some(b"de".as_slice()));
    let set_cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect::<Vec<_>>()
        .join(";");
    assert!(
        set_cookie.contains("locale=de"),
        "set-cookie={set_cookie}"
    );
}

#[tokio::test]
async fn enable_all_json_false_hides_all() {
    let dir = tempfile::tempdir().unwrap();
    write_locales(dir.path());
    let mut app = App::new();
    app.install(
        I18n::new(dir.path(), vec![Locale::new("en")])
            .path_prefix(false)
            .enable_all_json(false),
    );
    let all = app
        .handle_request(Method::GET, "/_i18n/all.json", "")
        .await;
    assert_eq!(all.status_code().as_u16(), 404);
}

#[tokio::test]
async fn invalid_locale_json_fails_startup() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("en.json"), "[1,2,3]").unwrap();
    let mut app = App::new();
    app.install(I18n::new(dir.path(), vec![Locale::new("en")]));
    assert!(app.run_startup().await.is_err());
}
