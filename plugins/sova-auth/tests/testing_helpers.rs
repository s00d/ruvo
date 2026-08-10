use sova_auth::testing::{ensure_permission, ensure_role, ActingAs, UserFactory};
use sova_auth::AuthMigrator;
use sova_core::{Request, Response, ResponseAssert, TestClient};
use sova_testing::{SqliteTestDb, TestApp};

#[tokio::test]
async fn user_factory_and_acting_as() {
    let (tdb, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .configure(|app| {
            app.get("/me", |req: Request| async move {
                let email = req
                    .get::<sova_auth::CurrentUser>()
                    .map(|u| u.email.clone())
                    .unwrap_or_default();
                Response::json(&serde_json::json!({ "email": email }))
            });
        })
        .build()
        .await;

    let db = tdb.handle().await;
    ensure_role(&db, "Editor", "editor").await;
    ensure_role(&db, "Editor", "editor").await;
    ensure_permission(&db, "Edit posts", "posts.edit").await;
    ensure_permission(&db, "Edit posts", "posts.edit").await;

    let user = UserFactory::new()
        .email("factory@example.com")
        .name("Factory")
        .password("password123")
        .roles(["editor"])
        .create(&db)
        .await;
    assert!(user.roles.iter().any(|r| r == "editor"));

    let c = TestClient::tracked(app).await.unwrap();
    c.acting_as(user);
    let res = c.get("/me").await;
    res.assert_status(200);
    assert_eq!(res.json_value()["email"], "factory@example.com");
}

#[tokio::test]
async fn sqlite_test_db_migrate_only() {
    let tdb = SqliteTestDb::migrate::<AuthMigrator>().await;
    let db = tdb.handle().await;
    let user = UserFactory::new()
        .email("solo@example.com")
        .create(&db)
        .await;
    assert_eq!(user.email, "solo@example.com");
    assert!(user.roles.iter().any(|r| r == "user"));
}
