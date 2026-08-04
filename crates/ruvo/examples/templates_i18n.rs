//! MiniJinja templates with ambient i18n `t(...)`.

use ruvo::{Bind, init_tracing, App, I18n, I18nExt, Locale, Plugin, RenderExt, Request, Result, Templates, template_fn};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let locales_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/i18n_locales");
    let views_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/templates_i18n/views");

    let mut app = App::new();
    I18n::new(
        &locales_dir,
        vec![Locale::new("en").with_name("English"), Locale::new("de").with_name("Deutsch")],
    )
    .fallback("en")
    .path_prefix(false)
    .install(&mut app);

    app.install(
        Templates::minijinja(&views_dir).per_request("t", template_fn),
    );

    app.get("/", |req: Request| async move {
        req.render(
            "home.html",
            serde_json::json!({
                "title": "Ruvo templates + i18n",
                "locale": req.locale(),
            }),
        )
        .unwrap_or_else(|e| e.into_response())
    });

    app.bind(Bind::Port(3006)).serve().await
}
