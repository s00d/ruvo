//! i18n example: SSR page + JSON catalog for the frontend.

use ruvo::prelude::*;
use ruvo::{mount_localized, I18n, I18nExt, I18nRouteExt, Locale, PrefixMode, Plugin};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let locales_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/i18n_locales");
    let locales = vec![
        Locale::new("en").with_name("English"),
        Locale::new("de").with_name("Deutsch"),
    ];
    let codes: Vec<String> = locales.iter().map(|l| l.code.clone()).collect();

    let mut app = App::new();
    I18n::new(&locales_dir, locales)
        .fallback("en")
        .path_prefix(true)
        .install(&mut app);

    mount_localized(&mut app, &codes, "en", PrefixMode::Prefix, |r| {
        r.get("/", |req: Request| async move {
            let html = format!(
                "<!doctype html><html lang=\"{lang}\"><body>\
                 <h1>{title}</h1><p>{about}</p>\
                 <p><a href=\"/_i18n/{lang}/blog.json\">/_i18n/{lang}/blog.json</a></p>\
                 </body></html>",
                lang = req.locale(),
                title = req.t("title"),
                about = req.t("nav.about"),
            );
            Html(html)
        })
        .i18n_scope("blog");
    });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    println!("i18n example on http://127.0.0.1:{port} (try /en/ and /de/)");
    app.listen(port).await
}
