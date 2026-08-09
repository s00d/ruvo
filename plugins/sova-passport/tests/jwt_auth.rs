//! JwtAuth routes, guard, from_env, PAT CRUD (offline sqlite).

#![cfg(feature = "jwt")]

use sova_core::{Json, Request, ResponseAssert, Router, TestClient};
use sova_passport::{AuthMigrator, JwtAuth, JwtAuthExt};
use sova_testing::TestApp;

#[tokio::test]
async fn register_login_refresh_logout_roundtrip() {
    let (_tdb, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .install(
            JwtAuth::hs256("test-secret-at-least-32-bytes!!")
                .access_ttl(60)
                .refresh_ttl(3600)
                .mount("/auth"),
        )
        .build()
        .await;

    let c = TestClient::tracked(app).await.unwrap();

    let reg = c
        .post("/auth/register")
        .header("content-type", "application/json")
        .body(r#"{"email":"ada@example.com","password":"secret123"}"#)
        .await;
    reg.assert_status(201);
    let pair = reg.json_value();
    assert!(!pair["access_token"].as_str().unwrap().is_empty());
    let refresh = pair["refresh_token"].as_str().unwrap().to_string();

    let login = c
        .post("/auth/login")
        .header("content-type", "application/json")
        .body(r#"{"email":"ada@example.com","password":"secret123"}"#)
        .await;
    login.assert_status(200);

    let bad = c
        .post("/auth/login")
        .header("content-type", "application/json")
        .body(r#"{"email":"ada@example.com","password":"wrong"}"#)
        .await;
    assert_eq!(bad.status_code().as_u16(), 401);

    let refreshed = c
        .post("/auth/refresh")
        .header("content-type", "application/json")
        .body(format!(r#"{{"refresh_token":"{refresh}"}}"#))
        .await;
    refreshed.assert_status(200);
    let new_refresh = refreshed.json_value()["refresh_token"]
        .as_str()
        .unwrap()
        .to_string();

    // Old refresh revoked after rotate.
    let reuse = c
        .post("/auth/refresh")
        .header("content-type", "application/json")
        .body(format!(r#"{{"refresh_token":"{refresh}"}}"#))
        .await;
    assert_ne!(reuse.status_code().as_u16(), 200);

    let out = c
        .post("/auth/logout")
        .header("content-type", "application/json")
        .body(format!(r#"{{"refresh_token":"{new_refresh}"}}"#))
        .await;
    out.assert_status(200);
    assert_eq!(out.body_bytes(), Some(b"ok".as_slice()));
}

#[tokio::test]
async fn guard_rejects_missing_and_bad_bearer() {
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

    let c = TestClient::tracked(app).await.unwrap();
    c.get("/api/me").await.assert_status(401);
    c.get("/api/me")
        .header("authorization", "Bearer not-a-jwt")
        .await
        .assert_status(401);
}

#[tokio::test]
async fn guard_rejects_jwt_for_unknown_user_sub() {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

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

    let token = encode(
        &Header::default(),
        &Claims {
            sub: "99999".into(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        },
        &EncodingKey::from_secret(b"test-secret-at-least-32-bytes!!"),
    )
    .unwrap();

    let c = TestClient::tracked(app).await.unwrap();
    c.get("/api/me")
        .header("authorization", format!("Bearer {token}"))
        .await
        .assert_status(401);
}

#[tokio::test]
async fn pat_crud_via_http_and_api_token_ext() {
    let (_tdb, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .install(
            JwtAuth::hs256("test-secret-at-least-32-bytes!!")
                .tokens(true)
                .tokens_mount("/auth/tokens"),
        )
        .configure(|app| {
            let mut api = Router::new();
            api.use_middleware(JwtAuth::guard());
            api.get("/who", |req: Request| async move {
                let via = if req.api_token().is_some() {
                    "pat"
                } else {
                    "jwt"
                };
                Ok::<_, sova_core::Error>(Json(serde_json::json!({ "via": via })))
            });
            app.mount("/api", api);
        })
        .build()
        .await;

    let c = TestClient::tracked(app).await.unwrap();
    c.post("/auth/register")
        .header("content-type", "application/json")
        .body(r#"{"email":"pat@example.com","password":"secret123"}"#)
        .await
        .assert_status(201);

    let login = c
        .post("/auth/login")
        .header("content-type", "application/json")
        .body(r#"{"email":"pat@example.com","password":"secret123"}"#)
        .await;
    login.assert_status(200);
    let access = login.json_value()["access_token"]
        .as_str()
        .unwrap()
        .to_string();

    let created = c
        .post("/auth/tokens")
        .header("authorization", format!("Bearer {access}"))
        .header("content-type", "application/json")
        .body(r#"{"name":"ci","abilities":["read"]}"#)
        .await;
    created.assert_status(201);
    let token = created.json_value()["token"].as_str().unwrap().to_string();
    let id = created.json_value()["id"].as_i64().unwrap();

    let list = c
        .get("/auth/tokens")
        .header("authorization", format!("Bearer {access}"))
        .await;
    list.assert_status(200);
    assert_eq!(list.json_value().as_array().unwrap().len(), 1);

    let who = c
        .get("/api/who")
        .header("authorization", format!("Bearer {token}"))
        .await;
    who.assert_status(200);
    assert_eq!(who.json_value()["via"], "pat");

    c.delete(format!("/auth/tokens/{id}"))
        .header("authorization", format!("Bearer {access}"))
        .await
        .assert_status(200);

    c.get("/api/who")
        .header("authorization", format!("Bearer {token}"))
        .await
        .assert_status(401);

    c.delete("/auth/tokens/99999")
        .header("authorization", format!("Bearer {access}"))
        .await
        .assert_status(404);
}

#[tokio::test]
async fn tokens_disabled_skips_pat_routes() {
    let (_tdb, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .install(JwtAuth::hs256("test-secret-at-least-32-bytes!!").tokens(false))
        .build()
        .await;

    let c = TestClient::tracked(app).await.unwrap();
    c.get("/auth/tokens").await.assert_status(404);
}

#[tokio::test]
async fn from_env_reads_ttl_and_empty_secret_fails_startup() {
    std::env::set_var("JWT_SECRET", "env-secret-at-least-32-bytes-ok!");
    std::env::set_var("JWT_ACCESS_TTL", "120");
    std::env::set_var("JWT_REFRESH_TTL", "7200");
    let auth = JwtAuth::from_env().mount("/jwt");
    let (_tdb, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .install(auth)
        .build()
        .await;
    let c = TestClient::tracked(app).await.unwrap();
    c.post("/jwt/register")
        .header("content-type", "application/json")
        .body(r#"{"email":"env@example.com","password":"secret123"}"#)
        .await
        .assert_status(201);

    std::env::set_var("JWT_SECRET", "");
    let empty = JwtAuth::from_env();
    let mut app2 = sova_core::App::new();
    // Db required by JwtAuth::requires — install empty-secret plugin alone;
    // install returns early with on_startup error.
    use sova_core::Plugin;
    empty.install(&mut app2);
    let err = app2.run_startup().await;
    assert!(err.is_err(), "empty JWT_SECRET must fail startup");
}

#[tokio::test]
async fn duplicate_register_is_conflict() {
    let (_tdb, app) = TestApp::builder()
        .migrator::<AuthMigrator>()
        .install(JwtAuth::hs256("test-secret-at-least-32-bytes!!"))
        .build()
        .await;
    let c = TestClient::tracked(app).await.unwrap();
    let body = r#"{"email":"dup@example.com","password":"secret123"}"#;
    c.post("/auth/register")
        .header("content-type", "application/json")
        .body(body)
        .await
        .assert_status(201);
    let again = c
        .post("/auth/register")
        .header("content-type", "application/json")
        .body(body)
        .await;
    assert_eq!(again.status_code().as_u16(), 409);
}
