//! Fortify login route uses RateLimit::login (429 after N attempts).

use serde_json::json;
use sova_auth::{AuthMigrator, Feature, Fortify};
use sova_core::TestClient;
use sova_mail::Mail;
use sova_session::memory_sessions;
use sova_testing::TestApp;

#[tokio::test]
async fn login_throttle_returns_429() {
    let (_db, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .env("FORTIFY_SECRET", "test-fortify-secret-login-throttle!!")
        .install(memory_sessions())
        .install(Mail::fake().from("Test <noreply@test.local>"))
        .install(
            Fortify::new()
                .features([Feature::Registration])
                .web_forms(false)
                .api_mount("/api/auth")
                .public_url("http://127.0.0.1")
                .app_name("Test")
                .home("/")
                .login_redirect("/login"),
        )
        .build()
        .await;

    let c = TestClient::tracked(app).await.unwrap();

    for i in 0..5 {
        let res = c
            .post("/api/auth/login")
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .json(&json!({
                "email": "victim@example.com",
                "password": "wrong-password"
            }))
            .await;
        assert_ne!(
            res.status_code().as_u16(),
            429,
            "attempt {i} should not be throttled yet"
        );
    }

    let blocked = c
        .post("/api/auth/login")
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .json(&json!({
            "email": "victim@example.com",
            "password": "wrong-password"
        }))
        .await;
    assert_eq!(blocked.status_code().as_u16(), 429);
    assert!(blocked.headers().get("ratelimit-limit").is_some());
}
