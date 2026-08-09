//! Sova kitchen-sink demo: Fortify auth, cabinet, notes, most web plugins.
//!
//! ```bash
//! export DATABASE_URL=postgres://postgres@localhost/sova
//! cargo run -p cabinet -- migrate
//! cargo run -p cabinet -- seed
//! cargo run -p cabinet
//! # http://127.0.0.1:3000  demo@sova.local / demo1234
//! ```
//!
//! Or via tooling: `cargo sovax db migrate -p cabinet` / `cargo sovax db seed -p cabinet`.

mod db;
mod entity;
mod migrate;
mod modules;
mod seed;
mod state;

use migrate::CabinetMigrator;
use sova::{
    bearer_guard, logger, namespace, priority, store, tasks, with_flash, Activity, App,
    AuthFeature, Channel, Compress, Cors, Csrf, Db, DbPool, Dispatch, Fortify, I18n, Job,
    Locale, Mail, Meta, Notifications, OpenApi, OutboundHttp, Parser, RateLimit,
    RateLimitKey, Result, Robots, ServerArgs, SessionLayer, SharedStore, Shield, Sitemap, Static,
    Storage, Tasks, Templates, Vld, Ws, Observability, request_id,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = sova::sova_env::load();
    let args = ServerArgs::parse();
    args.init_tracing();

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let public_url = std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".into());
    let tasks_bearer =
        std::env::var("TASKS_BEARER").unwrap_or_else(|_| "cabinet-dev-secret".into());

    let mut app = App::new();
    // Kitchen-sink lives under examples/cabinet — load its sova.toml explicitly.
    let _ = app.configure_from_path(root.join("sova.toml"));
    let bus = app.events();
    bus.listen::<modules::notes::NoteCreated, _>(|e| {
        tracing::info!(note_id = e.note_id, user_id = e.user_id, "note.created");
    });
    bus.listen::<sova::UserRegistered, _>(|e| {
        tracing::info!(user_id = %e.user_id, email = %e.email, "auth.user_registered");
    });
    bus.listen::<sova::UserLoggedIn, _>(|e| {
        tracing::info!(user_id = %e.user_id, email = %e.email, "auth.user_logged_in");
    });
    bus.listen::<sova::MailSent, _>(|e| {
        tracing::info!(to = ?e.to, subject = %e.subject, "mail.sent");
    });
    bus.listen::<sova::CsrfMismatch, _>(|e| {
        tracing::warn!(method = %e.method, path = %e.path, "csrf.mismatch");
    });
    bus.listen::<sova::TaskFailed, _>(|e| {
        tracing::warn!(id = %e.id, name = %e.name, attempts = e.attempts, "tasks.failed");
    });
    app.use_middleware(request_id());
    app.install(Observability::new());
    app.use_middleware(logger());

    app.install(
        Db::from_env()
            .migrations::<CabinetMigrator>()
            .seed(|state| async move { seed::seed_demo(state).await }),
    );
    let db_pool = app
        .try_state::<DbPool>()
        .expect("Db plugin inserts DbPool")
        .as_ref()
        .clone();
    let kv = Arc::new(store::Sql::from_db_pool(&db_pool));
    let task_store = Arc::new(tasks::Sql::from_db_pool(&db_pool));

    app.install(Cors::new());
    app.install(Shield::new());
    app.install(Compress::new());
    app.install(SharedStore::new(Arc::clone(&kv) as Arc<dyn sova::KvStore>));
    app.install(SessionLayer::from_store(Arc::new(sova::SqlSessionStore::from_db_pool(
        &db_pool,
    ))));
    // Session cookie auth: CSRF on forms + /api/auth; except tasks API.
    app.install(Csrf::new().except("/_tasks/*"));
    app.install(
        RateLimit::fixed_window(
            Arc::new(namespace(Arc::clone(&kv) as Arc<dyn sova::KvStore>, "rl")),
            120,
            Duration::from_secs(60),
        )
        .key(RateLimitKey::Identity),
    );

    app.install(Static::new("/assets", root.join("public")));
    // path/public_url also in sova.toml [storage]; local root stays code (absolute).
    app.install(Storage::local(root.join("public").join("uploads")));

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
    let templates = with_flash(
        Templates::minijinja(&views).per_request("t", sova::template_fn),
    );
    app.install(templates);

    // site_name / title_template / public_url from [meta] in sova.toml (unset-fill).
    app.install(Meta::new().public_url(&public_url));
    app.install(
        Sitemap::new()
            .public_url(&public_url)
            .exclude("/cabinet/*")
            .exclude("/api/*")
            .exclude("/_tasks/*")
            .exclude("/docs/*")
            .exclude("/user/*")
            .exclude("/admin/*")
            .exclude("/activity")
            .exclude("/notifications")
            .exclude("/ws/notifications"),
    );
    app.install(Robots::new().disallow("/cabinet").disallow("/api"));

    app.install(Vld);
    app.install(OpenApi::new("Cabinet API", "0.1.0"));
    app.install(Ws::new());
    app.install(OutboundHttp::new());

    // `from` comes from [mail] when not set on the builder.
    let mail_plugin = Mail::from_env();
    let mail = mail_plugin.client();
    app.install(mail_plugin);

    app.install(
        Tasks::new(task_store)
            .queues(["default", "mailer"])
            .job(
                Job::new("welcome_email", move |task| {
                    let mail = mail.clone();
                    async move {
                        let data = serde_json::from_slice::<serde_json::Value>(&task.payload)
                            .ok()
                            .and_then(|v| v.get("data").cloned())
                            .unwrap_or_else(|| serde_json::json!({}));
                        let email = data
                            .get("email")
                            .and_then(|e| e.as_str())
                            .unwrap_or("unknown@example.com")
                            .to_string();
                        let name = data
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("there")
                            .to_string();
                        let link = std::env::var("PUBLIC_URL")
                            .unwrap_or_else(|_| "http://127.0.0.1:3000".into());
                        mail.compose()
                            .to(email.clone())
                            .subject("Welcome to Cabinet")
                            .text(format!(
                                "Hello {name}! Your account ({email}) is ready.\n{link}"
                            ))
                            .view(
                                "mail/welcome.html",
                                serde_json::json!({
                                    "name": name,
                                    "link": link,
                                }),
                            )
                            .send()
                            .await
                            .map_err(|e| e.to_string())?;
                        tracing::info!(%email, "welcome_email sent");
                        Ok(())
                    }
                })
                .queue("mailer")
                .priority(priority::LOW),
            )
            .exposed()
            .guard(bearer_guard(tasks_bearer)),
    );

    app.install(
        Fortify::new()
            .features(AuthFeature::all().iter().copied())
            .public_url(&public_url)
            .app_name("Sova Cabinet")
            .home("/cabinet")
            .login_redirect("/login")
            .profile_path("/cabinet/profile")
            .api_mount("/api/auth")
            .after_register(|user, req| async move {
                if let Some(tasks) = req.try_state::<sova::TaskBackend>() {
                    let _ = tasks
                        .dispatch(
                            Dispatch::new("welcome_email")
                                .data(serde_json::json!({ "email": user.email, "name": user.name })),
                        )
                        .await;
                }
                Ok(req)
            }),
    );

    app.install(
        Activity::new()
            .mount("/activity")
            .guard(Fortify::permission("users.manage")),
    );

    app.install(
        Notifications::new()
            .channel(
                Channel::new("orders").publish("notifications.orders.publish"),
            )
            .channel(Channel::new("security"))
            .mount("/notifications")
            .guard(Fortify::guard())
            .ws_path("/ws/notifications")
            .with_template_helpers(),
    );

    modules::register(&mut app);
    app.with_probes();

    tracing::info!("cabinet demo — open {public_url} (after migrate+seed: demo@sova.local / demo1234)");
    app.run().await
}
