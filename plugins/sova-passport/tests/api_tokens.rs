//! PAT create / verify / revoke / JwtAuth::guard dual auth.

#![cfg(feature = "jwt")]

use sova_core::{Json, Request, ResponseAssert, Router, TestClient};
use sova_db::{DbHandle, DbPool};
use sova_passport::{
    create_api_token, list_api_tokens, register_user, revoke_api_token, user_for_api_token,
    AuthMigrator, CreateApiToken, JwtAuth, JwtAuthExt,
};
use sova_testing::{SqliteTestDb, TestApp};

#[tokio::test]
async fn create_list_revoke_and_verify() {
    let tdb = SqliteTestDb::migrate::<AuthMigrator>().await;
    let db = tdb.handle().await;

    let user = register_user(&db, "ada@example.com", "secret123")
        .await
        .unwrap();

    let created = create_api_token(
        &db,
        user.id,
        CreateApiToken {
            name: "ci".into(),
            abilities: vec!["read".into()],
            expires_at: None,
        },
    )
    .await
    .unwrap();
    assert!(created.token.starts_with("svpat_"));

    let list = list_api_tokens(&db, user.id).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "ci");
    let json = serde_json::to_string(&list).unwrap();
    assert!(!json.contains(&created.token));

    let (u, info) = user_for_api_token(&db, &created.token).await.unwrap();
    assert_eq!(u.id, user.id);
    assert_eq!(info.abilities, vec!["read".to_string()]);

    assert!(revoke_api_token(&db, user.id, created.id, None).await.unwrap());
    assert!(user_for_api_token(&db, &created.token).await.is_err());
}

#[tokio::test]
async fn guard_accepts_pat_and_jwt() {
    let (_tdb, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .install(JwtAuth::hs256("test-secret-at-least-32-bytes!!"))
        .configure(|app| {
            let mut api = Router::new();
            api.use_middleware(JwtAuth::guard());
            api.get("/me", |req: Request| async move {
                Ok::<_, sova_core::Error>(Json(req.require_auth_user()?.clone()))
            });
            app.mount("/api", api);
        })
        .build()
        .await;

    let pool = app.try_state::<DbPool>().expect("DbPool").as_ref().clone();
    let conn = pool.get().unwrap();
    let db = DbHandle::Conn(conn);
    let user = register_user(&db, "bob@example.com", "secret123")
        .await
        .unwrap();
    let pat = create_api_token(
        &db,
        user.id,
        CreateApiToken {
            name: "bot".into(),
            abilities: vec![],
            expires_at: None,
        },
    )
    .await
    .unwrap();

    let c = TestClient::tracked(app).await.unwrap();

    c.get("/api/me")
        .header("authorization", format!("Bearer {}", pat.token))
        .await
        .assert_status(200);

    let login = c
        .post("/auth/login")
        .header("content-type", "application/json")
        .body(r#"{"email":"bob@example.com","password":"secret123"}"#)
        .await;
    login.assert_status(200);
    let access = login.json_value()["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    c.get("/api/me")
        .header("authorization", format!("Bearer {access}"))
        .await
        .assert_status(200);
}
