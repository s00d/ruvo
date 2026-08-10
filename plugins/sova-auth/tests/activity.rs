//! Fortify + activity: successful profile update writes `profile.updated`.

#![cfg(feature = "activity")]

use sea_orm_migration::MigratorTrait;
use serde_json::json;
use sova_activity::{list_activity, Activity, ActivityFilter, ActivityMigrator};
use sova_auth::{AuthMigrator, Feature, Fortify};
use sova_core::TestClient;
use sova_mail::Mail;
use sova_session::memory_sessions;
use sova_testing::TestApp;

struct CombinedMigrator;

#[async_trait::async_trait]
impl MigratorTrait for CombinedMigrator {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        let mut v = AuthMigrator::migrations();
        v.extend(ActivityMigrator::migrations());
        v
    }
}

#[tokio::test]
async fn profile_update_writes_activity() {
    let (tdb, app) = TestApp::builder()
        .migrator::<CombinedMigrator>()
        .env("FORTIFY_SECRET", "test-fortify-secret-for-activity!!")
        .install(memory_sessions())
        .install(Mail::fake().from("Test <noreply@test.local>"))
        .install(Activity::new())
        .install(
            Fortify::new()
                .features([Feature::Registration, Feature::UpdateProfile])
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

    let reg = c
        .post("/api/auth/register")
        .header("accept", "application/json")
        .json(&json!({
            "name": "Ada",
            "email": "ada@example.com",
            "password": "secret123",
            "password_confirmation": "secret123"
        }))
        .await;
    assert!(
        (200..300).contains(&reg.status_code().as_u16()),
        "register: {}",
        reg.status_code()
    );

    let upd = c
        .post("/api/auth/profile")
        .header("accept", "application/json")
        .json(&json!({
            "name": "Ada Lovelace",
            "email": "ada@example.com"
        }))
        .await;
    assert!(
        (200..300).contains(&upd.status_code().as_u16()),
        "profile: {}",
        upd.status_code()
    );

    let db = tdb.handle().await;
    let rows = list_activity(
        &db,
        ActivityFilter {
            event: Some("profile.updated".into()),
            subject_type: Some("user".into()),
            subject_id: None,
            actor_id: None,
            limit: 10,
        },
    )
    .await
    .unwrap();
    assert!(
        !rows.is_empty(),
        "expected profile.updated activity row, got {rows:?}"
    );
    assert!(rows[0].properties.get("name").is_some());
}
