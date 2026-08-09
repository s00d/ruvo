//! Offline Fortify JSON API coverage (`/api/auth/*`).

use sova_auth::{assign_role, AuthMigrator, Feature, Fortify};
use sova_core::{ClientRequest, ResponseAssert, TestClient};
use sova_mail::{FakeMail, Mail};
use sova_session::memory_sessions;
use sova_testing::{SqliteTestDb, TestApp};
use serde_json::{json, Value};
use totp_rs::{Algorithm, Secret, TOTP};

const SECRET: &str = "test-fortify-secret-fortify-api!!";

async fn build() -> (SqliteTestDb, TestClient, FakeMail) {
    let mail = Mail::fake().from("Test <noreply@test.local>");
    let fake = mail.recorder().expect("fake recorder").clone();
    let (_db, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .env("FORTIFY_SECRET", SECRET)
        .install(memory_sessions())
        .install(mail)
        .install(
            Fortify::new()
                .features(Feature::all().iter().copied())
                .web_forms(false)
                .api_mount("/api/auth")
                .public_url("http://127.0.0.1")
                .app_name("Test")
                .home("/")
                .login_redirect("/login")
                .secret(SECRET),
        )
        .build()
        .await;
    let c = TestClient::tracked(app).await.expect("test client");
    (_db, c, fake)
}

fn json_headers(req: ClientRequest<'_>) -> ClientRequest<'_> {
    req.header("accept", "application/json")
}

async fn register(c: &TestClient, name: &str, email: &str, password: &str) -> Value {
    let res = json_headers(c.post("/api/auth/register"))
        .json(&json!({
            "name": name,
            "email": email,
            "password": password,
            "password_confirmation": password,
        }))
        .await;
    assert!(
        (200..300).contains(&res.status_code().as_u16()),
        "register {}: {}",
        email,
        res.status_code()
    );
    res.json_value()
}

async fn login(c: &TestClient, email: &str, password: &str) -> sova_core::Response {
    json_headers(c.post("/api/auth/login"))
        .json(&json!({ "email": email, "password": password }))
        .await
}

fn query_param(haystack: &str, key: &str) -> String {
    let url = haystack
        .split_whitespace()
        .find(|w| w.contains("http://") || w.contains("https://"))
        .unwrap_or(haystack);
    let Some(q) = url.split('?').nth(1) else {
        panic!("no query in mail body: {haystack}");
    };
    for pair in q.split('&') {
        let mut it = pair.splitn(2, '=');
        if it.next() == Some(key) {
            let raw = it.next().unwrap_or("");
            return raw
                .replace("%40", "@")
                .replace("%2E", ".")
                .replace("%2e", ".");
        }
    }
    panic!("missing query key `{key}` in: {haystack}");
}

fn totp_now(secret_b32: &str) -> String {
    let secret = Secret::Encoded(secret_b32.to_string())
        .to_bytes()
        .expect("secret");
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some("Sova".into()),
        "user".into(),
    )
    .expect("totp");
    totp.generate_current().expect("code")
}

#[tokio::test]
async fn register_success_and_duplicate_email() {
    let (_db, c, _fake) = build().await;

    let body = register(&c, "Ada", "ada@example.com", "secret123").await;
    assert_eq!(body["email"], "ada@example.com");
    assert_eq!(body["name"], "Ada");
    assert!(body["id"].as_i64().is_some());

    let dup = json_headers(c.post("/api/auth/register"))
        .json(&json!({
            "name": "Other",
            "email": "ada@example.com",
            "password": "secret123",
            "password_confirmation": "secret123",
        }))
        .await;
    assert_eq!(dup.status_code().as_u16(), 409);
}

#[tokio::test]
async fn login_success_wrong_password_and_logout() {
    let (_db, c, _fake) = build().await;
    register(&c, "Bob", "bob@example.com", "secret123").await;

    json_headers(c.post("/api/auth/logout"))
        .json(&json!({}))
        .await
        .assert_status(200);

    let bad = login(&c, "bob@example.com", "wrong-password").await;
    assert_eq!(bad.status_code().as_u16(), 401);

    let ok = login(&c, "bob@example.com", "secret123").await;
    ok.assert_status(200);
    assert_eq!(ok.json_value()["email"], "bob@example.com");

    let me = json_headers(c.get("/api/auth/me")).await;
    me.assert_status(200);
    assert_eq!(me.json_value()["email"], "bob@example.com");

    let out = json_headers(c.post("/api/auth/logout"))
        .json(&json!({}))
        .await;
    out.assert_status(200);
    assert_eq!(out.json_value()["ok"], true);

    let me2 = json_headers(c.get("/api/auth/me")).await;
    assert_eq!(me2.status_code().as_u16(), 401);
}

#[tokio::test]
async fn profile_get_and_update() {
    let (_db, c, _fake) = build().await;
    register(&c, "Cara", "cara@example.com", "secret123").await;

    let me = json_headers(c.get("/api/auth/profile")).await;
    me.assert_status(200);
    assert_eq!(me.json_value()["name"], "Cara");

    let upd = json_headers(c.post("/api/auth/profile"))
        .json(&json!({
            "name": "Cara Updated",
            "email": "cara@example.com",
        }))
        .await;
    upd.assert_status(200);
    assert_eq!(upd.json_value()["ok"], true);

    let me2 = json_headers(c.get("/api/auth/me")).await;
    me2.assert_status(200);
    assert_eq!(me2.json_value()["name"], "Cara Updated");
}

#[tokio::test]
async fn password_update() {
    let (_db, c, _fake) = build().await;
    register(&c, "Dan", "dan@example.com", "secret123").await;

    let bad = json_headers(c.post("/api/auth/password"))
        .json(&json!({
            "current_password": "nope",
            "password": "newsecret1",
            "password_confirmation": "newsecret1",
        }))
        .await;
    assert_eq!(bad.status_code().as_u16(), 400);

    let ok = json_headers(c.post("/api/auth/password"))
        .json(&json!({
            "current_password": "secret123",
            "password": "newsecret1",
            "password_confirmation": "newsecret1",
        }))
        .await;
    ok.assert_status(200);
    assert_eq!(ok.json_value()["ok"], true);

    json_headers(c.post("/api/auth/logout"))
        .json(&json!({}))
        .await
        .assert_status(200);

    login(&c, "dan@example.com", "secret123")
        .await
        .assert_status(401);
    login(&c, "dan@example.com", "newsecret1")
        .await
        .assert_status(200);
}

#[tokio::test]
async fn forgot_and_reset_password_via_fake_mail() {
    let (_db, c, fake) = build().await;
    register(&c, "Eve", "eve@example.com", "secret123").await;
    json_headers(c.post("/api/auth/logout"))
        .json(&json!({}))
        .await
        .assert_status(200);
    fake.clear();

    let forgot = json_headers(c.post("/api/auth/forgot-password"))
        .json(&json!({ "email": "eve@example.com" }))
        .await;
    forgot.assert_status(200);
    assert_eq!(forgot.json_value()["ok"], true);

    let sent = fake.sent();
    assert_eq!(sent.len(), 1, "expected reset mail");
    let body = sent[0]
        .text
        .as_deref()
        .or(sent[0].html.as_deref())
        .expect("mail body");
    let token = query_param(body, "token");
    let email = query_param(body, "email");
    assert_eq!(email, "eve@example.com");

    let reset = json_headers(c.post("/api/auth/reset-password"))
        .json(&json!({
            "email": "eve@example.com",
            "token": token,
            "password": "resetpass1",
            "password_confirmation": "resetpass1",
        }))
        .await;
    reset.assert_status(200);
    assert_eq!(reset.json_value()["ok"], true);

    login(&c, "eve@example.com", "secret123")
        .await
        .assert_status(401);
    login(&c, "eve@example.com", "resetpass1")
        .await
        .assert_status(200);
}

#[tokio::test]
async fn email_verification_via_fake_mail() {
    let (_db, c, fake) = build().await;
    fake.clear();

    register(&c, "Fay", "fay@example.com", "secret123").await;

    let sent = fake.sent();
    assert!(
        !sent.is_empty(),
        "expected verify mail on register with EmailVerification"
    );
    let body = sent[0]
        .text
        .as_deref()
        .or(sent[0].html.as_deref())
        .expect("mail body");
    let token = query_param(body, "token");

    let me = json_headers(c.get("/api/auth/me")).await;
    me.assert_status(200);
    assert_eq!(me.json_value()["email_verified"], false);

    let verify = json_headers(c.post("/api/auth/email/verify"))
        .json(&json!({ "token": token }))
        .await;
    verify.assert_status(200);
    assert_eq!(verify.json_value()["ok"], true);

    // Session CurrentUser may still show old flag until re-login.
    json_headers(c.post("/api/auth/logout"))
        .json(&json!({}))
        .await
        .assert_status(200);
    login(&c, "fay@example.com", "secret123")
        .await
        .assert_status(200);
    let me2 = json_headers(c.get("/api/auth/me")).await;
    me2.assert_status(200);
    assert_eq!(me2.json_value()["email_verified"], true);
}

#[tokio::test]
async fn two_factor_enable_confirm_challenge_disable() {
    let (_db, c, _fake) = build().await;
    register(&c, "Gus", "gus@example.com", "secret123").await;

    let en = json_headers(c.post("/api/auth/two-factor"))
        .json(&json!({}))
        .await;
    en.assert_status(200);
    let en_body = en.json_value();
    let secret = en_body["secret"].as_str().expect("secret");
    assert!(en_body["recovery_codes"].as_array().is_some());

    let code = totp_now(secret);
    let confirm = json_headers(c.post("/api/auth/two-factor/confirm"))
        .json(&json!({ "code": code }))
        .await;
    confirm.assert_status(200);
    assert_eq!(confirm.json_value()["ok"], true);

    let me = json_headers(c.get("/api/auth/me")).await;
    me.assert_status(200);
    assert_eq!(me.json_value()["two_factor_enabled"], true);

    json_headers(c.post("/api/auth/logout"))
        .json(&json!({}))
        .await
        .assert_status(200);

    let challenge_login = login(&c, "gus@example.com", "secret123").await;
    challenge_login.assert_status(200);
    assert_eq!(challenge_login.json_value()["two_factor"], true);

    let me_pending = json_headers(c.get("/api/auth/me")).await;
    assert_eq!(me_pending.status_code().as_u16(), 401);

    let challenge = json_headers(c.post("/api/auth/two-factor/challenge"))
        .json(&json!({ "code": totp_now(secret) }))
        .await;
    challenge.assert_status(200);
    assert_eq!(challenge.json_value()["email"], "gus@example.com");

    let disable = json_headers(c.post("/api/auth/two-factor/disable"))
        .json(&json!({ "password": "secret123" }))
        .await;
    disable.assert_status(200);
    assert_eq!(disable.json_value()["ok"], true);

    let me2 = json_headers(c.get("/api/auth/me")).await;
    me2.assert_status(200);
    assert_eq!(me2.json_value()["two_factor_enabled"], false);
}

#[tokio::test]
async fn roles_crud_as_admin() {
    let (tdb, c, _fake) = build().await;
    let body = register(&c, "Admin", "admin@example.com", "secret123").await;
    let uid = body["id"].as_i64().unwrap();

    let denied = json_headers(c.get("/api/auth/roles")).await;
    assert_eq!(denied.status_code().as_u16(), 403);

    let db = tdb.handle().await;
    assign_role(&db, uid, "admin").await.expect("assign admin");
    json_headers(c.post("/api/auth/logout"))
        .json(&json!({}))
        .await
        .assert_status(200);
    login(&c, "admin@example.com", "secret123")
        .await
        .assert_status(200);

    let list = json_headers(c.get("/api/auth/roles")).await;
    list.assert_status(200);
    assert!(list.json_value()["roles"].as_array().unwrap().len() >= 2);

    let created = json_headers(c.post("/api/auth/roles"))
        .json(&json!({ "name": "Editor", "slug": "editor" }))
        .await;
    assert_eq!(created.status_code().as_u16(), 201);
    let role_id = created.json_value()["role"]["id"].as_i64().unwrap();

    let show = json_headers(c.get(format!("/api/auth/roles/{role_id}"))).await;
    show.assert_status(200);
    assert_eq!(show.json_value()["role"]["slug"], "editor");

    let perms = json_headers(c.get("/api/auth/permissions")).await;
    perms.assert_status(200);
    let perm_id = perms.json_value()["permissions"][0]["id"]
        .as_i64()
        .expect("perm id");

    let sync = json_headers(c.put(format!("/api/auth/roles/{role_id}/permissions")))
        .json(&json!({ "permission_ids": [perm_id] }))
        .await;
    sync.assert_status(200);

    // Assign editor to a *different* user so the admin session keeps users.manage.
    let other = register(&c, "Other", "other@example.com", "secret123").await;
    let other_id = other["id"].as_i64().unwrap();
    // register logs in as other — switch back to admin
    json_headers(c.post("/api/auth/logout"))
        .json(&json!({}))
        .await
        .assert_status(200);
    login(&c, "admin@example.com", "secret123")
        .await
        .assert_status(200);

    let user_roles = json_headers(c.get(format!("/api/auth/users/{other_id}/roles"))).await;
    user_roles.assert_status(200);

    let set_roles = json_headers(c.put(format!("/api/auth/users/{other_id}/roles")))
        .json(&json!({ "role_ids": [role_id] }))
        .await;
    set_roles.assert_status(200);

    let del = json_headers(c.delete(format!("/api/auth/roles/{role_id}"))).await;
    del.assert_status(200);
}

#[tokio::test]
async fn confirm_password_status() {
    let (_db, c, _fake) = build().await;
    register(&c, "Hal", "hal@example.com", "secret123").await;

    let st = json_headers(c.get("/api/auth/confirmed-password-status")).await;
    st.assert_status(200);
    assert_eq!(st.json_value()["confirmed"], false);

    let conf = json_headers(c.post("/api/auth/confirm-password"))
        .json(&json!({ "password": "secret123" }))
        .await;
    conf.assert_status(200);
    assert_eq!(conf.json_value()["confirmed"], true);

    let st2 = json_headers(c.get("/api/auth/confirmed-password-status")).await;
    st2.assert_status(200);
    assert_eq!(st2.json_value()["confirmed"], true);
}
