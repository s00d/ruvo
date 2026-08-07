//! Ruvo kitchen-sink demo: Fortify auth, cabinet, notes, most web plugins.
//!
//! ```bash
//! export DATABASE_URL=postgres://postgres@localhost/ruvo
//! cargo run -p cabinet -- migrate
//! cargo run -p cabinet
//! # http://127.0.0.1:3000  demo@ruvo.local / demo1234
//! ```

mod db;
mod entity;
mod migrate;
mod modules;
mod seed;
mod state;

use migrate::CabinetMigrator;
use ruvo::{
    bearer_guard, logger, namespace, store, tasks, with_validation_flash, App, AuthFeature, Compress,
    Cors, Csrf, Db, DbPool, Email, Fortify, I18n, Locale, Mail, Meta, OpenApi, OutboundHttp, Parser,
    RateLimit, Result, Robots, ServerArgs, SessionLayer, SharedStore, Shield, Sitemap, Static,
    Tasks, Templates, Vld, Ws,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = ruvo::ruvo_env::load();
    let args = ServerArgs::parse();
    args.init_tracing();

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let public_url = std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".into());
    let tasks_bearer =
        std::env::var("TASKS_BEARER").unwrap_or_else(|_| "cabinet-dev-secret".into());

    let mut app = App::new();
    app.use_middleware(logger());

    app.install(Db::from_env().migrations::<CabinetMigrator>());
    let db_pool = app
        .try_state::<DbPool>()
        .expect("Db plugin inserts DbPool")
        .as_ref()
        .clone();
    let kv = Arc::new(store::Sql::from_db_pool(&db_pool));
    let task_store = Arc::new(tasks::Sql::from_db_pool(&db_pool));

    app.install(Cors::new().origin("*"));
    app.install(Shield::new());
    app.install(Compress::new());
    app.install(SharedStore::new(Arc::clone(&kv) as Arc<dyn ruvo::KvStore>));
    app.install(SessionLayer::new(Arc::new(namespace(
        Arc::clone(&kv) as Arc<dyn ruvo::KvStore>,
        "sess",
    ))));
    // Session cookie auth: CSRF on forms + /api/auth; except tasks API.
    app.install(Csrf::new().except("/_tasks/*"));
    app.install(RateLimit::fixed_window(
        Arc::new(namespace(Arc::clone(&kv) as Arc<dyn ruvo::KvStore>, "rl")),
        120,
        Duration::from_secs(60),
    ));

    app.install(Static::new("/assets", root.join("public")));

    let locales = root.join("locales");
    app.install(
        I18n::new(
            &locales,
            vec![
                Locale::new("en").with_name("English"),
                Locale::new("ru").with_name("Русский"),
            ],
        )
        .fallback("en")
        .path_prefix(false)
        .cookie("locale")
        .set_locale_cookie(true),
    );

    let views = root.join("views");
    let templates = with_validation_flash(
        Templates::minijinja(&views)
            .autoreload(cfg!(debug_assertions))
            .per_request("t", ruvo::template_fn),
    );
    app.install(templates);

    app.install(
        Meta::new()
            .site_name("Ruvo Cabinet")
            .title_template("{} — Ruvo Cabinet")
            .public_url(&public_url),
    );
    app.install(
        Sitemap::new()
            .public_url(&public_url)
            .exclude("/cabinet/*")
            .exclude("/api/*")
            .exclude("/_tasks/*")
            .exclude("/docs/*")
            .exclude("/user/*")
            .exclude("/admin/*"),
    );
    app.install(Robots::new().disallow("/cabinet").disallow("/api"));

    app.install(Vld);
    app.install(OpenApi::new("Cabinet API", "0.1.0").mount("/docs"));
    app.install(Ws::new());
    app.install(OutboundHttp::new());

    let mail_plugin = Mail::from_env().from("Cabinet <noreply@ruvo.local>");
    let mail = mail_plugin.client();
    app.install(mail_plugin);

    app.install(
        Tasks::new(task_store)
            .on("welcome_email", move |task| {
                let mail = mail.clone();
                async move {
                    let email = serde_json::from_slice::<serde_json::Value>(&task.payload)
                        .ok()
                        .and_then(|v| {
                            v.get("data")
                                .and_then(|d| d.get("email"))
                                .and_then(|e| e.as_str())
                                .map(str::to_string)
                        })
                        .unwrap_or_else(|| "unknown@example.com".into());
                    mail.send(
                        Email::new()
                            .to(email.clone())
                            .subject("Welcome to Cabinet")
                            .text(format!("Hello! Your account ({email}) is ready."))
                            .html(format!(
                                "<p>Hello!</p><p>Your account (<strong>{email}</strong>) is ready.</p>"
                            )),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    tracing::info!(%email, "welcome_email sent");
                    Ok(())
                }
            })
            .exposed()
            .guard(bearer_guard(tasks_bearer)),
    );

    app.install(
        Fortify::new()
            .features(AuthFeature::all().iter().copied())
            .public_url(&public_url)
            .app_name("Ruvo Cabinet")
            .home("/cabinet")
            .login_redirect("/login")
            .profile_path("/cabinet/profile")
            .api_mount("/api/auth")
            .after_register(|user, req| async move {
                if let Some(tasks) = req.try_state::<ruvo::TaskBackend>() {
                    let _ = tasks
                        .enqueue(
                            "default",
                            "welcome_email",
                            serde_json::json!({ "email": user.email, "name": user.name }),
                        )
                        .await;
                }
                Ok(req)
            }),
    );

    app.on_startup(seed::seed_demo);

    modules::register(&mut app);

    tracing::info!("cabinet demo — open {public_url} (demo@ruvo.local / demo1234)");
    app.run().await
}
