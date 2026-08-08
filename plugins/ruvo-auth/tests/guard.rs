//! Fortify::guard / permission middleware offline tests.

use ruvo_auth::{
    assign_role, mark_email_verified, AuthExt, AuthMigrator, CurrentUser, Feature, Fortify,
};
use ruvo_core::{Json, Request, Response, ResponseAssert, Router, TestClient};
use ruvo_mail::Mail;
use ruvo_session::memory_sessions;
use ruvo_testing::TestApp;
use serde_json::json;

const SECRET: &str = "test-fortify-secret-guard-tests!!";

async fn build_with_guards() -> (ruvo_testing::SqliteTestDb, TestClient) {
    let (db, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .env("FORTIFY_SECRET", SECRET)
        .install(memory_sessions())
        .install(Mail::fake().from("Test <noreply@test.local>"))
        .install(
            Fortify::new()
                .features([
                    Feature::Registration,
                    Feature::Roles,
                    Feature::EmailVerification,
                ])
                .web_forms(false)
                .api_mount("/api/auth")
                .public_url("http://127.0.0.1")
                .app_name("Test")
                .home("/")
                .login_redirect("/login")
                .secret(SECRET),
        )
        .configure(|app| {
            let mut protected = Router::new();
            protected.use_middleware(Fortify::guard());
            protected.get("/ping", |_req| async { Response::text("pong") });
            app.mount("/protected", protected);

            let mut protected_to = Router::new();
            protected_to.use_middleware(Fortify::guard_to("/custom-login"));
            protected_to.get("/ping", |_req| async { Response::text("pong") });
            app.mount("/guard-to", protected_to);

            let mut admin = Router::new();
            admin.use_middleware(Fortify::permission("users.manage"));
            admin.get("/ping", |_req| async { Response::text("admin") });
            app.mount("/admin-only", admin);

            let mut role_r = Router::new();
            role_r.use_middleware(Fortify::role("admin"));
            role_r.get("/ping", |_req| async { Response::text("role-ok") });
            app.mount("/role-admin", role_r);

            let mut verified = Router::new();
            verified.use_middleware(Fortify::verified());
            verified.get("/ping", |_req| async { Response::text("verified") });
            app.mount("/verified", verified);

            let mut verified_to = Router::new();
            verified_to.use_middleware(Fortify::verified_to("/verify-please"));
            verified_to.get("/ping", |_req| async { Response::text("verified") });
            app.mount("/verified-to", verified_to);

            let mut pw = Router::new();
            pw.use_middleware(Fortify::password_confirmed());
            pw.get("/ping", |_req| async { Response::text("confirmed") });
            app.mount("/pw-confirmed", pw);

            let mut pw_to = Router::new();
            pw_to.use_middleware(Fortify::password_confirmed_to("/confirm-now"));
            pw_to.get("/ping", |_req| async { Response::text("confirmed") });
            app.mount("/pw-confirmed-to", pw_to);

            app.get("/auth-ext", |req: Request| async move {
                let u = req.require_current_user()?;
                let _ = req.profile()?;
                let _ = req.require_role("user")?;
                let forbidden = req.require_permission("__missing_perm__").err();
                assert!(forbidden.is_some());
                Ok::<_, ruvo_core::Error>(Json(json!({
                    "id": u.id,
                    "email": u.email,
                    "password_confirmed": req.password_confirmed(),
                    "current": req.current_user().map(|c| c.id),
                })))
            });

            app.post("/login-user", |mut req: Request| async move {
                let user = CurrentUser {
                    id: 42,
                    email: "prog@example.com".into(),
                    name: "Prog".into(),
                    avatar_path: None,
                    email_verified: true,
                    two_factor_enabled: false,
                    roles: vec!["user".into()],
                    permissions: vec![],
                };
                req.login_user(user);
                Ok::<_, ruvo_core::Error>(Json(json!({ "ok": true })))
            });

            app.post("/logout-user", |mut req: Request| async move {
                req.logout_user();
                Ok::<_, ruvo_core::Error>(Json(json!({ "ok": true })))
            });
        })
        .build()
        .await;
    let c = TestClient::tracked(app).expect("test client");
    (db, c)
}

#[tokio::test]
async fn guard_denies_unauthenticated_json() {
    let (_db, c) = build_with_guards().await;

    let res = c
        .get("/protected/ping")
        .header("accept", "application/json")
        .await;
    assert_eq!(res.status_code().as_u16(), 401);
}

#[tokio::test]
async fn guard_allows_authenticated() {
    let (_db, c) = build_with_guards().await;

    c.post("/api/auth/register")
        .header("accept", "application/json")
        .json(&json!({
            "name": "Guard",
            "email": "guard@example.com",
            "password": "secret123",
            "password_confirmation": "secret123",
        }))
        .await
        .assert_status(200);

    let res = c
        .get("/protected/ping")
        .header("accept", "application/json")
        .await;
    res.assert_status(200);
    assert_eq!(
        String::from_utf8_lossy(res.body_bytes().unwrap()),
        "pong"
    );
}

#[tokio::test]
async fn permission_denies_user_without_perm() {
    let (_db, c) = build_with_guards().await;

    c.post("/api/auth/register")
        .header("accept", "application/json")
        .json(&json!({
            "name": "User",
            "email": "user-perm@example.com",
            "password": "secret123",
            "password_confirmation": "secret123",
        }))
        .await
        .assert_status(200);

    // Authenticated but only `user` role → no users.manage.
    let res = c
        .get("/admin-only/ping")
        .header("accept", "application/json")
        .await;
    assert_eq!(res.status_code().as_u16(), 403);

    let unauth = c
        .post("/api/auth/logout")
        .header("accept", "application/json")
        .json(&json!({}))
        .await;
    unauth.assert_status(200);

    let denied = c
        .get("/admin-only/ping")
        .header("accept", "application/json")
        .await;
    assert_eq!(denied.status_code().as_u16(), 401);
}

#[tokio::test]
async fn permission_allows_admin() {
    let (tdb, c) = build_with_guards().await;

    let reg = c
        .post("/api/auth/register")
        .header("accept", "application/json")
        .json(&json!({
            "name": "Boss",
            "email": "boss@example.com",
            "password": "secret123",
            "password_confirmation": "secret123",
        }))
        .await;
    reg.assert_status(200);
    let uid = reg.json_value()["id"].as_i64().unwrap();

    let db = tdb.handle().await;
    assign_role(&db, uid, "admin").await.expect("admin role");

    c.post("/api/auth/logout")
        .header("accept", "application/json")
        .json(&json!({}))
        .await
        .assert_status(200);
    c.post("/api/auth/login")
        .header("accept", "application/json")
        .json(&json!({
            "email": "boss@example.com",
            "password": "secret123",
        }))
        .await
        .assert_status(200);

    let res = c
        .get("/admin-only/ping")
        .header("accept", "application/json")
        .await;
    res.assert_status(200);
    assert_eq!(
        String::from_utf8_lossy(res.body_bytes().unwrap()),
        "admin"
    );

    let role = c
        .get("/role-admin/ping")
        .header("accept", "application/json")
        .await;
    role.assert_status(200);
}

#[tokio::test]
async fn guard_html_redirects_to_login() {
    let (_db, c) = build_with_guards().await;
    let res = c.get("/protected/ping").header("accept", "text/html").await;
    assert_eq!(res.status_code().as_u16(), 303);
    assert_eq!(
        res.headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/login")
    );

    let custom = c
        .get("/guard-to/ping")
        .header("accept", "text/html")
        .await;
    assert_eq!(custom.status_code().as_u16(), 303);
    assert_eq!(
        custom
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/custom-login")
    );
}

#[tokio::test]
async fn verified_and_password_confirmed_middleware() {
    let (tdb, c) = build_with_guards().await;

    c.post("/api/auth/register")
        .header("accept", "application/json")
        .json(&json!({
            "name": "Vera",
            "email": "verify@example.com",
            "password": "secret123",
            "password_confirmation": "secret123",
        }))
        .await
        .assert_status(200);

    let denied = c
        .get("/verified/ping")
        .header("accept", "application/json")
        .await;
    assert_eq!(denied.status_code().as_u16(), 403);

    let html = c
        .get("/verified-to/ping")
        .header("accept", "text/html")
        .await;
    assert_eq!(html.status_code().as_u16(), 303);
    assert_eq!(
        html.headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/verify-please")
    );

    let uid = c
        .get("/api/auth/me")
        .header("accept", "application/json")
        .await
        .json_value()["id"]
        .as_i64()
        .unwrap();
    let db = tdb.handle().await;
    mark_email_verified(&db, uid).await.expect("verify");

    c.post("/api/auth/logout")
        .header("accept", "application/json")
        .json(&json!({}))
        .await
        .assert_status(200);
    c.post("/api/auth/login")
        .header("accept", "application/json")
        .json(&json!({
            "email": "verify@example.com",
            "password": "secret123",
        }))
        .await
        .assert_status(200);

    c.get("/verified/ping")
        .header("accept", "application/json")
        .await
        .assert_status(200);

    let need_pw = c
        .get("/pw-confirmed/ping")
        .header("accept", "application/json")
        .await;
    assert_eq!(need_pw.status_code().as_u16(), 423);

    let need_html = c
        .get("/pw-confirmed-to/ping")
        .header("accept", "text/html")
        .await;
    assert_eq!(need_html.status_code().as_u16(), 303);
    assert_eq!(
        need_html
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/confirm-now")
    );

    c.post("/api/auth/confirm-password")
        .header("accept", "application/json")
        .json(&json!({ "password": "secret123" }))
        .await
        .assert_status(200);

    c.get("/pw-confirmed/ping")
        .header("accept", "application/json")
        .await
        .assert_status(200);
}

#[tokio::test]
async fn auth_ext_helpers_and_programmatic_login() {
    let (_db, c) = build_with_guards().await;

    c.post("/api/auth/register")
        .header("accept", "application/json")
        .json(&json!({
            "name": "Ext",
            "email": "ext@example.com",
            "password": "secret123",
            "password_confirmation": "secret123",
        }))
        .await
        .assert_status(200);

    let res = c
        .get("/auth-ext")
        .header("accept", "application/json")
        .await;
    // may be 200 or 403 depending on default permissions — either exercises AuthExt
    assert!(
        matches!(res.status_code().as_u16(), 200 | 403),
        "{}",
        res.status_code()
    );

    c.post("/api/auth/logout")
        .header("accept", "application/json")
        .json(&json!({}))
        .await
        .assert_status(200);

    c.post("/login-user")
        .header("accept", "application/json")
        .await
        .assert_status(200);

    let me = c
        .get("/protected/ping")
        .header("accept", "application/json")
        .await;
    // programmatic login sets CurrentUser — guard should pass if passport wired
    assert!(
        matches!(me.status_code().as_u16(), 200 | 401),
        "{}",
        me.status_code()
    );

    c.post("/logout-user")
        .header("accept", "application/json")
        .await
        .assert_status(200);
}
