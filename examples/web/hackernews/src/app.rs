use crate::migrate::HnMigrator;
use crate::modules;
use crate::seed;
use sova::prelude::*;
use sova::{AuthFeature, Db, Fortify, Result};
use std::path::PathBuf;

/// Build the HN app (web preset + sqlite + Fortify registration).
pub fn build_app() -> Result<App> {
    build_app_with_db(None)
}

/// Same as [`build_app`], with an optional pinned `DATABASE_URL` (for parallel tests).
pub fn build_app_with_db(database_url: Option<&str>) -> Result<App> {
    let _ = sova::sova_env::load();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let public_url =
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".into());

    let mut app = App::web()
        .site("Sova News")
        .public_url(&public_url)
        .views(root.join("views"))
        .assets(root.join("public"))
        .into_app();
    let _ = app.configure_from_path(root.join("sova.toml"));

    let mut db = Db::from_env()
        .migrations::<HnMigrator>()
        .migrate_on_startup()
        .seed(|state| async move { seed::run(state).await })
        .seed_on_startup();
    if let Some(url) = database_url {
        db = db.url(url);
    }
    app.install(db);

    app.install(
        Fortify::new()
            .features([AuthFeature::Registration])
            .web_forms(true)
            .no_api()
            .home("/")
            .login_redirect("/login")
            .public_url(&public_url)
            .app_name("Sova News")
            .secret(
                std::env::var("FORTIFY_SECRET")
                    .unwrap_or_else(|_| "dev-hn-secret-change-me".into()),
            ),
    );

    modules::register(&mut app);
    Ok(app)
}
