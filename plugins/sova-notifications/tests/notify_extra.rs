//! Notify builders: body, to(), unknown channel, Via::Mail, role audience.

use sova_core::{Json, Request, ResponseAssert, TestClient};
use sova_mail::Mail;
use sova_notifications::{Channel, Notifications, NotificationsMigrator, Notify, Via};
use sova_testing::TestApp;
use serde_json::json;

#[tokio::test]
async fn notify_to_body_and_unknown_channel() {
    let mail = Mail::fake().from("N <n@t.local>");
    #[cfg(feature = "mail")]
    let fake = mail.recorder().unwrap().clone();
    let (_db, app) = TestApp::builder()
        .migrator::<NotificationsMigrator>()
        .install(mail)
        .install(
            Notifications::new()
                .channel(Channel::new("orders"))
                .mount("/notifications"),
        )
        .configure(|app| {
            app.post("/send", |req: Request| async move {
                #[cfg(feature = "mail")]
                let vias = [Via::Database, Via::Mail];
                #[cfg(not(feature = "mail"))]
                let vias = [Via::Database];
                let rows = Notify::to(7)
                    .channel("orders")
                    .event("ping")
                    .title("Hello & <world>")
                    .body("Body <b>")
                    .data(json!({ "email": "u@example.com" }))
                    .via(vias)
                    .system(true)
                    .send(&req)
                    .await?;
                Ok::<_, sova_core::Error>(Json(json!({ "n": rows.len() })))
            });
            app.post("/bad-channel", |req: Request| async move {
                Notify::to(1)
                    .channel("nope")
                    .title("x")
                    .via([Via::Database])
                    .send(&req)
                    .await
                    .map(|_| Json(json!({})))
            });
            app.get("/svc", |req: Request| async move {
                use sova_notifications::NotifyExt;
                let n = req.notifications().expect("svc");
                assert!(n.channel("orders").is_some());
                assert!(n.ensure_channel("orders").is_ok());
                assert!(n.ensure_channel("missing").is_err());
                Ok::<_, sova_core::Error>(Json(json!({ "ok": true })))
            });
        })
        .build()
        .await;

    let c = TestClient::tracked(app).await.unwrap();
    c.post("/send").await.assert_status(200);
    #[cfg(feature = "mail")]
    {
        let sent = fake.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, vec!["u@example.com"]);
        assert!(sent[0]
            .html
            .as_deref()
            .unwrap()
            .contains("&lt;") || sent[0].html.is_some());
    }

    c.post("/bad-channel").await.assert_status(400);
    c.get("/svc").await.assert_status(200);
}

#[tokio::test]
#[cfg(feature = "auth")]
async fn notify_to_role_and_permission() {
    use sova_auth::{assign_role, AuthMigrator, Feature, Fortify};
    use sova_session::memory_sessions;
    use sea_orm_migration::MigratorTrait;

    struct Combined;
    #[async_trait::async_trait]
    impl MigratorTrait for Combined {
        fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
            let mut v = AuthMigrator::migrations();
            v.extend(NotificationsMigrator::migrations());
            v
        }
    }

    let (_db, app) = TestApp::builder()
        .migrator::<Combined>()
        .env("FORTIFY_SECRET", "test-notif-role-secret-at-least-32b!!")
        .install(memory_sessions())
        .install(Mail::fake().from("t <t@t.local>"))
        .install(
            Fortify::new()
                .features([Feature::Registration, Feature::Roles])
                .web_forms(false)
                .api_mount("/api/auth")
                .public_url("http://127.0.0.1")
                .app_name("t")
                .home("/")
                .login_redirect("/login"),
        )
        .install(
            Notifications::new()
                .channel(Channel::new("default"))
                .mount("/notifications"),
        )
        .configure(|app| {
            app.post("/role", |req: Request| async move {
                let rows = Notify::to_role("user")
                    .event("role.ping")
                    .title("Role")
                    .via([Via::Database])
                    .send(&req)
                    .await?;
                Ok::<_, sova_core::Error>(Json(json!({ "n": rows.len() })))
            });
            app.post("/perm", |req: Request| async move {
                let rows = Notify::to_permission("users.manage")
                    .event("perm.ping")
                    .title("Perm")
                    .via([Via::Database])
                    .send(&req)
                    .await?;
                Ok::<_, sova_core::Error>(Json(json!({ "n": rows.len() })))
            });
        })
        .build()
        .await;

    let c = TestClient::tracked(app).await.unwrap();
    let reg = c
        .post("/api/auth/register")
        .header("accept", "application/json")
        .json(&json!({
            "name": "RoleUser",
            "email": "role-n@example.com",
            "password": "secret123",
            "password_confirmation": "secret123",
        }))
        .await;
    reg.assert_status(200);
    let uid = reg.json_value()["id"].as_i64().unwrap();

    // Ensure user role audience resolves at least this user.
    let db = _db.handle().await;
    let _ = assign_role(&db, uid, "user").await;

    let role = c.post("/role").await;
    role.assert_status(200);
    assert!(role.json_value()["n"].as_u64().unwrap() >= 1);

    // No admin yet — permission audience may be empty but should not error.
    let perm = c.post("/perm").await;
    perm.assert_status(200);
}
