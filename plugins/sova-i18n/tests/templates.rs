//! Templates + i18n integration (`template_fn`).

use http::Method;
use sova_core::{App, Request};
use sova_i18n::{template_fn, I18n, Locale};
use sova_templates::{RenderExt, Templates};

fn write_locales(dir: &std::path::Path) {
    std::fs::write(
        dir.join("en.json"),
        r#"{
            "nav": { "about": "About" },
            "greet": "Hello {name}",
            "cart.items": "none|one|many"
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
}

#[tokio::test]
async fn template_fn_translates_plural_and_interpolation() {
    let locales_dir = tempfile::tempdir().unwrap();
    write_locales(locales_dir.path());

    let views = tempfile::tempdir().unwrap();
    std::fs::write(
        views.path().join("page.html"),
        r#"<p>{{ t("nav.about") }}|{{ t("greet", name="Ada") }}|{{ t("cart.items", count=2) }}</p>"#,
    )
    .unwrap();

    let mut app = App::new();
    app.install(
        I18n::new(
            locales_dir.path(),
            vec![Locale::new("en"), Locale::new("de")],
        )
        .fallback("en")
        .path_prefix(false),
    );
    app.install(
        Templates::minijinja(views.path())
            .autoreload(false)
            .per_request("t", template_fn),
    );

    app.get("/en", |req: Request| async move {
        req.render("page.html", serde_json::json!({}))
            .unwrap_or_else(|e| e.into_response())
    });
    app.get("/de", |req: Request| async move {
        req.render("page.html", serde_json::json!({}))
            .unwrap_or_else(|e| e.into_response())
    });

    let en = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/en")
                .header("accept-language", "en")
                .build(),
        )
        .await;
    let body = std::str::from_utf8(en.body_bytes().unwrap()).unwrap();
    assert!(body.contains("About|Hello Ada|many"), "{body}");

    let de = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/de")
                .header("accept-language", "de")
                .build(),
        )
        .await;
    let body = std::str::from_utf8(de.body_bytes().unwrap()).unwrap();
    assert!(body.contains("Über|Hallo Ada|viele"), "{body}");
}

#[tokio::test]
async fn template_fn_local_ctx_overrides_ambient_t() {
    let locales_dir = tempfile::tempdir().unwrap();
    write_locales(locales_dir.path());

    let views = tempfile::tempdir().unwrap();
    std::fs::write(
        views.path().join("page.html"),
        r#"<p>{{ t("nav.about") }}|{{ local_label }}</p>"#,
    )
    .unwrap();

    let mut app = App::new();
    app.install(I18n::new(locales_dir.path(), vec![Locale::new("en")]).path_prefix(false));
    app.install(
        Templates::minijinja(views.path())
            .autoreload(false)
            .global("local_label", "from-global")
            .per_request("local_label", |_| minijinja::Value::from("from-request"))
            .per_request("t", template_fn),
    );

    app.get("/", |req: Request| async move {
        req.render(
            "page.html",
            serde_json::json!({ "local_label": "from-handler" }),
        )
        .unwrap_or_else(|e| e.into_response())
    });

    let res = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("accept-language", "en")
                .build(),
        )
        .await;
    let body = std::str::from_utf8(res.body_bytes().unwrap()).unwrap();
    assert!(body.contains("About|from-handler"), "{body}");
}
