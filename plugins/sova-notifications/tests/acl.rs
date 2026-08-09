//! Broadcast publish ACL (feature `auth`).

#![cfg(feature = "auth")]

use sova_auth::{AuthMigrator, CurrentUser, Feature, Fortify};
use sova_core::{ResponseAssert, TestClient};
use sova_mail::Mail;
use sova_notifications::{Channel, Notifications, NotificationsMigrator};
use sova_session::memory_sessions;
use sova_testing::{ActingAs, TestApp};
use sea_orm_migration::MigratorTrait;
use serde_json::json;

struct Combined;

#[async_trait::async_trait]
impl MigratorTrait for Combined {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        let mut v = AuthMigrator::migrations();
        v.extend(NotificationsMigrator::migrations());
        v
    }
}

#[tokio::test]
async fn broadcast_denied_without_publish_permission() {
    let (_db, app) = TestApp::builder()
        .migrator::<Combined>()
        .env("FORTIFY_SECRET", "test-notif-acl-secret-at-least-32b!!")
        .install(memory_sessions())
        .install(Mail::fake().from("t <t@t.local>"))
        .install(
            Fortify::new()
                .features([Feature::Registration])
                .web_forms(false)
                .api_mount("/api/auth")
                .public_url("http://127.0.0.1")
                .app_name("t")
                .home("/")
                .login_redirect("/login"),
        )
        .install(
            Notifications::new()
                .channel(Channel::new("orders").publish("notifications.orders.publish"))
                .mount("/notifications"),
        )
        .build()
        .await;

    let c = TestClient::tracked(app).await.unwrap();
    c.acting_as(CurrentUser {
        id: 99,
        email: "u@example.com".into(),
        name: "U".into(),
        avatar_path: None,
        email_verified: true,
        two_factor_enabled: false,
        roles: vec!["user".into()],
        permissions: vec!["cabinet.access".into()],
    });

    let res = c
        .post("/notifications/broadcast")
        .json(&json!({
            "channel": "orders",
            "event": "x",
            "title": "t",
            "audience": { "users": [1] }
        }))
        .await;
    res.assert_status(403);
}
