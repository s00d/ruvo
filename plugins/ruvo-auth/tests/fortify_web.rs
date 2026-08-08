//! Fortify `web_forms(true)` HTML routes + form POST (with CSRF).

use ruvo_auth::{AuthMigrator, Feature, Fortify};
use ruvo_core::{ResponseAssert, TestClient};
use ruvo_csrf::Csrf;
use ruvo_mail::Mail;
use ruvo_session::memory_sessions;
use ruvo_testing::{SqliteTestDb, TestApp};

const SECRET: &str = "test-fortify-secret-fortify-web!!";

async fn build() -> (SqliteTestDb, TestClient) {
    let (_db, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .env("FORTIFY_SECRET", SECRET)
        .install(memory_sessions())
        .install(Csrf::new())
        .install(Mail::fake().from("Test <noreply@test.local>"))
        .install(
            Fortify::new()
                .features(Feature::all().iter().copied())
                .web_forms(true)
                .no_api()
                .public_url("http://127.0.0.1")
                .app_name("Test")
                .home("/home")
                .login_redirect("/login")
                .secret(SECRET),
        )
        .build()
        .await;
    let c = TestClient::tracked(app).expect("test client");
    (_db, c)
}

fn csrf_from_html(body: &str) -> String {
    let marker = "name=\"csrf\" value=\"";
    let start = body
        .find(marker)
        .map(|i| i + marker.len())
        .expect("csrf hidden field");
    let rest = &body[start..];
    let end = rest.find('"').expect("csrf value end");
    rest[..end].to_string()
}

fn xsrf_cookie(res: &ruvo_core::Response) -> Option<String> {
    let prefix = "XSRF-TOKEN=";
    res.headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|c| c.starts_with(prefix))
        .map(|c| {
            c.split(';')
                .next()
                .unwrap()
                .trim_start_matches(prefix)
                .to_string()
        })
}

#[tokio::test]
async fn get_register_and_login_forms_return_html() {
    let (_db, c) = build().await;

    let reg = c.get("/register").await;
    reg.assert_status(200);
    let ct = reg
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.contains("text/html"), "content-type={ct}");
    let body = String::from_utf8_lossy(reg.body_bytes().unwrap()).into_owned();
    assert!(body.contains("<form"), "{body}");
    assert!(body.contains("name=\"email\""), "{body}");
    assert!(body.contains("name=\"password\""), "{body}");
    let csrf = csrf_from_html(&body);
    assert!(!csrf.is_empty());
    assert_eq!(xsrf_cookie(&reg).as_deref(), Some(csrf.as_str()));

    let login = c.get("/login").await;
    login.assert_status(200);
    let body = String::from_utf8_lossy(login.body_bytes().unwrap()).into_owned();
    assert!(body.contains("<form"), "{body}");
    assert!(body.contains("name=\"email\""), "{body}");
    assert!(!csrf_from_html(&body).is_empty());
}

#[tokio::test]
async fn post_register_and_login_urlencoded_with_csrf() {
    let (_db, c) = build().await;

    let reg_page = c.get("/register").await;
    reg_page.assert_status(200);
    let csrf = csrf_from_html(&String::from_utf8_lossy(reg_page.body_bytes().unwrap()));

    let registered = c
        .post("/register")
        .form(&[
            ("csrf", csrf.as_str()),
            ("name", "Ada"),
            ("email", "ada@example.com"),
            ("password", "secret123"),
            ("password_confirmation", "secret123"),
        ])
        .await;
    assert_eq!(
        registered.status_code().as_u16(),
        303,
        "register should redirect home"
    );
    assert_eq!(
        registered.headers().get("location").and_then(|v| v.to_str().ok()),
        Some("/home")
    );

    // Logout via web POST (needs fresh csrf from a GET).
    let login_page = c.get("/login").await;
    let csrf = csrf_from_html(&String::from_utf8_lossy(login_page.body_bytes().unwrap()));
    c.post("/logout")
        .form(&[("csrf", csrf.as_str())])
        .await
        .assert_status(303);

    let login_page = c.get("/login").await;
    let csrf = csrf_from_html(&String::from_utf8_lossy(login_page.body_bytes().unwrap()));
    let bad = c
        .post("/login")
        .form(&[
            ("csrf", csrf.as_str()),
            ("email", "ada@example.com"),
            ("password", "wrong-password"),
        ])
        .await;
    // vld flash redirect or query-error redirect to /login
    assert_eq!(bad.status_code().as_u16(), 303);

    let login_page = c.get("/login").await;
    let csrf = csrf_from_html(&String::from_utf8_lossy(login_page.body_bytes().unwrap()));
    let ok = c
        .post("/login")
        .form(&[
            ("csrf", csrf.as_str()),
            ("email", "ada@example.com"),
            ("password", "secret123"),
        ])
        .await;
    assert_eq!(ok.status_code().as_u16(), 303);
    assert_eq!(
        ok.headers().get("location").and_then(|v| v.to_str().ok()),
        Some("/home")
    );
}

#[tokio::test]
async fn post_register_without_csrf_is_forbidden() {
    let (_db, c) = build().await;
    // Establish session + XSRF cookie.
    c.get("/register").await.assert_status(200);

    let res = c
        .post("/register")
        .form(&[
            ("name", "Bob"),
            ("email", "bob@example.com"),
            ("password", "secret123"),
            ("password_confirmation", "secret123"),
        ])
        .await;
    assert_eq!(res.status_code().as_u16(), 403);
}
