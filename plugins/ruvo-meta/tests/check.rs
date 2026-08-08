//! Meta `register_audit("meta")` — soft/strict titles, duplicates, images, staging.

use http::Method;
use ruvo_core::{App, CheckKind, Request, Response};
use ruvo_meta::{Meta, Robots};
use std::sync::Mutex;

/// Serialize env mutation — `RUVO_PROFILE` is process-global.
static ENV_LOCK: Mutex<()> = Mutex::new(());

async fn audit_meta(app: App) -> (bool, Option<String>) {
    audit_meta_profile(app, None).await
}

async fn audit_meta_profile(app: App, profile: Option<&str>) -> (bool, Option<String>) {
    let _guard = ENV_LOCK.lock().unwrap();
    match profile {
        Some(p) => std::env::set_var("RUVO_PROFILE", p),
        None => std::env::remove_var("RUVO_PROFILE"),
    }
    let server = app.build().unwrap();
    let results = app
        .run_checks(server.state(), &[CheckKind::Audit])
        .await;
    std::env::remove_var("RUVO_PROFILE");
    let meta = results
        .into_iter()
        .find(|r| r.name == "meta")
        .expect("meta audit registered");
    (meta.ok, meta.error)
}

#[tokio::test]
async fn soft_check_allows_missing_description() {
    let mut app = App::new();
    app.install(Meta::new().soft_check().site_name("S").public_url("https://ex.com"));
    app.get("/bare", |_r: Request| async { Response::text("ok") });

    let (ok, err) = audit_meta(app).await;
    assert!(ok, "soft check should warn only: {err:?}");
}

#[tokio::test]
async fn soft_check_via_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[meta]
check = "soft"
site_name = "S"
public_url = "https://ex.com"
"#,
    )
    .unwrap();
    app.install(Meta::new());
    app.get("/bare", |_r: Request| async { Response::text("ok") });

    let (ok, _) = audit_meta(app).await;
    assert!(ok);
}

#[tokio::test]
async fn strict_check_fails_missing_meta() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.get("/bare", |_r: Request| async { Response::text("ok") });

    let (ok, err) = audit_meta(app).await;
    assert!(!ok);
    let msg = err.unwrap_or_default();
    assert!(msg.contains("title+description"), "{msg}");
    assert!(msg.contains("GET /bare"), "{msg}");
}

#[tokio::test]
async fn strict_passes_with_title_and_description() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("About").description("About page"));

    let (ok, err) = audit_meta(app).await;
    assert!(ok, "{err:?}");
}

#[tokio::test]
async fn duplicate_canonical_paths_fail() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com").site_name("S"));
    app.get("/a", |_r: Request| async { Response::text("ok") })
        .with(
            Meta::page()
                .title("A")
                .description("d")
                .canonical_path("/same"),
        );
    app.get("/b", |_r: Request| async { Response::text("ok") })
        .with(
            Meta::page()
                .title("B")
                .description("d")
                .canonical_path("/same"),
        );

    let (ok, err) = audit_meta(app).await;
    assert!(!ok);
    let msg = err.unwrap_or_default();
    assert!(msg.contains("duplicate canonical"), "{msg}");
}

#[tokio::test]
async fn default_image_http_ok_missing_file_fails() {
    let mut ok_app = App::new();
    ok_app.install(
        Meta::new()
            .public_url("https://ex.com")
            .site_name("S")
            .default_image("https://cdn.example.com/og.png"),
    );
    ok_app
        .get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));
    let (ok, err) = audit_meta(ok_app).await;
    assert!(ok, "{err:?}");

    let mut bad = App::new();
    bad.install(
        Meta::new()
            .public_url("https://ex.com")
            .site_name("S")
            .default_image("/no/such/image-xyz.png"),
    );
    bad.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));
    let (ok, err) = audit_meta(bad).await;
    assert!(!ok);
    assert!(
        err.unwrap_or_default().contains("default_image"),
        "expected image error"
    );
}

#[tokio::test]
async fn staging_requires_robots_block_all() {
    let mut open = App::new();
    open.install(Meta::new().public_url("https://ex.com").site_name("S"));
    open.install(Robots::new());
    open.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));
    let (ok, err) = audit_meta_profile(open, Some("staging")).await;
    assert!(!ok);
    assert!(
        err.unwrap_or_default().contains("staging"),
        "expected staging robots error"
    );

    let mut blocked = App::new();
    blocked.install(Meta::new().public_url("https://ex.com").site_name("S"));
    blocked.install(Robots::new().block_all());
    blocked
        .get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));
    let (ok, err) = audit_meta_profile(blocked, Some("staging")).await;
    assert!(ok, "{err:?}");
}

#[tokio::test]
async fn noindex_and_dynamic_routes_skipped() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com"));
    app.get("/secret", |_r: Request| async { Response::text("ok") })
        .with(Meta::noindex());
    app.get("/items/:id", |_r: Request| async { Response::text("ok") });
    app.get("/about", |_r: Request| async { Response::text("ok") })
        .with(Meta::page().title("A").description("d"));

    let (ok, err) = audit_meta(app).await;
    assert!(ok, "{err:?}");
}

#[tokio::test]
async fn robots_txt_route_ignored_by_title_check() {
    let mut app = App::new();
    app.install(Meta::new().public_url("https://ex.com").site_name("S"));
    app.install(Robots::new());
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var("RUVO_PROFILE");
    let server = app.build().unwrap();
    let res = server
        .handle_request(Method::GET, "/robots.txt", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 200);

    let results = app
        .run_checks(server.state(), &[CheckKind::Audit])
        .await;
    let meta = results.iter().find(|r| r.name == "meta").unwrap();
    assert!(meta.ok, "{:?}", meta.error);
}
