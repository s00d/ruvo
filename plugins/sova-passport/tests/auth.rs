//! Passport strategies tests.

use http::Method;
use sova_core::{App, Json, Plugin, Request, Response};
use sova_passport::{Auth, Extract, Passport, PassportExt};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
struct User {
    id: i64,
    name: String,
}

#[tokio::test]
async fn bearer_sets_user() {
    let mut app = App::new();
    app.install(
        Passport::new().strategy(
            "bearer",
            Auth::bearer(|token, _req| async move {
                if token == "secret" {
                    Ok(Some(User {
                        id: 1,
                        name: "ada".into(),
                    }))
                } else {
                    Ok(None)
                }
            })
            .middleware(),
        ),
    );
    app.use_middleware(Passport::authenticate("bearer"));

    app.get("/me", |req: Request| async move {
        let u = req.require_user::<User>()?.clone();
        Ok::<_, sova_core::Error>(Json(u))
    });

    let ok = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/me")
                .header("authorization", "Bearer secret")
                .build(),
        )
        .await;
    assert_eq!(ok.status_code().as_u16(), 200);

    let bad = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/me")
                .header("authorization", "Bearer nope")
                .build(),
        )
        .await;
    assert_eq!(bad.status_code().as_u16(), 401);
}

#[tokio::test]
async fn api_key_via_passport() {
    let mut app = App::new();
    app.install(
        Passport::new().strategy(
            "api-key",
            Auth::api_key("x-api-key", |key, _| async move {
                Ok(if key == "k1" {
                    Some(User {
                        id: 2,
                        name: "key-user".into(),
                    })
                } else {
                    None
                })
            })
            .middleware(),
        ),
    );
    app.use_middleware(Passport::authenticate("api-key"));
    app.get("/me", |req: Request| async move {
        Ok::<_, sova_core::Error>(Json(req.require_user::<User>()?.clone()))
    });

    let ok = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/me")
                .header("x-api-key", "k1")
                .build(),
        )
        .await;
    assert_eq!(ok.status_code().as_u16(), 200);
}

#[tokio::test]
async fn skip_and_optional_on_strategy() {
    let mut app = App::new();
    Auth::bearer(|_, _| async {
        Ok(Some(User {
            id: 1,
            name: "u".into(),
        }))
    })
    .skip("/health")
    .install(&mut app);

    app.get("/health", |_r: Request| async { Response::text("ok") });
    app.get("/secret", |req: Request| async move {
        Ok::<_, sova_core::Error>(Json(req.require_user::<User>()?.clone()))
    });

    assert_eq!(
        app.handle(
            Request::builder()
                .method(Method::GET)
                .path("/health")
                .build()
        )
        .await
        .status_code()
        .as_u16(),
        200
    );
    assert_eq!(
        app.handle(
            Request::builder()
                .method(Method::GET)
                .path("/secret")
                .build()
        )
        .await
        .status_code()
        .as_u16(),
        401
    );
}

#[tokio::test]
async fn extract_chain_query() {
    let mut app = App::new();
    Auth::new()
        .extract(
            Extract::bearer()
                .or(Extract::header("x-api-key"))
                .or(Extract::query("api_key")),
        )
        .verify(|creds, _req| {
            let v = creds.value().to_string();
            async move {
                Ok(if v == "qkey" {
                    Some(User {
                        id: 3,
                        name: "query".into(),
                    })
                } else {
                    None
                })
            }
        })
        .install(&mut app);

    app.get("/me", |req: Request| async move {
        Ok::<_, sova_core::Error>(Json(req.require_user::<User>()?.clone()))
    });

    let ok = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/me")
                .query_param("api_key", "qkey")
                .build(),
        )
        .await;
    assert_eq!(ok.status_code().as_u16(), 200);
}

#[tokio::test]
async fn optional_continues() {
    let mut app = App::new();
    Auth::new()
        .extract(Extract::bearer())
        .optional(true)
        .verify(|creds, _req| {
            let v = creds.value().to_string();
            async move {
                Ok(if v == "ok" {
                    Some(User {
                        id: 9,
                        name: "opt".into(),
                    })
                } else {
                    None
                })
            }
        })
        .install(&mut app);

    app.get("/who", |req: Request| async move {
        let name = req
            .user::<User>()
            .map(|u| u.name.clone())
            .unwrap_or_else(|| "anon".into());
        Response::text(name)
    });

    let anon = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/who")
                .build(),
        )
        .await;
    assert_eq!(
        String::from_utf8_lossy(anon.body_bytes().unwrap_or_default()),
        "anon"
    );
}

#[cfg(feature = "jwt")]
#[tokio::test]
async fn jwt_hs256_helper() {
    use sova_passport::Jwt;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    let jwt = Jwt::hs256("test-secret");
    let token = jwt
        .encode(&Claims {
            sub: "42".into(),
            exp: 9_999_999_999,
        })
        .unwrap();

    let mut app = App::new();
    Auth::jwt_hs256("test-secret", |claims: Claims, _req| async move {
        Ok(Some(User {
            id: claims.sub.parse().unwrap_or(0),
            name: claims.sub,
        }))
    })
    .install(&mut app);

    app.get("/me", |req: Request| async move {
        Ok::<_, sova_core::Error>(Json(req.require_user::<User>()?.clone()))
    });

    let ok = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/me")
                .header("authorization", format!("Bearer {token}"))
                .build(),
        )
        .await;
    assert_eq!(ok.status_code().as_u16(), 200);
}

#[cfg(feature = "jwt")]
#[test]
fn jwt_issue_access_roundtrip() {
    use sova_passport::Jwt;
    let jwt = Jwt::hs256("roundtrip-secret");
    let token = jwt.issue_access("99", 120).unwrap();
    let claims = jwt.decode_access(&token).unwrap();
    assert_eq!(claims.sub, "99");
}

#[cfg(feature = "jwt")]
#[test]
fn password_and_refresh_hash() {
    use sova_passport::{hash_password, hash_refresh_token, verify_password};
    let hash = hash_password("secret123").unwrap();
    assert!(verify_password("secret123", &hash).unwrap());
    assert_eq!(hash_refresh_token("tok"), hash_refresh_token("tok"));
}

#[cfg(feature = "jwt")]
#[test]
fn auth_migrator_lists_migration() {
    use sova_db::MigratorTrait;
    use sova_passport::AuthMigrator;
    // Single init migration (users + refresh + api_tokens [+ oauth when feature on]).
    assert_eq!(AuthMigrator::migrations().len(), 1);
}

#[cfg(feature = "oauth")]
#[test]
fn oauth_state_and_pkce_roundtrip() {
    use sova_passport::oauth_test_support::{
        now_secs, pkce_challenge, random_urlsafe, sign_state, verify_state, FlowState,
    };
    let verifier = random_urlsafe(32);
    let challenge = pkce_challenge(&verifier);
    assert_ne!(challenge, verifier);
    let flow = FlowState {
        provider: "github".into(),
        code_verifier: verifier.clone(),
        nonce: random_urlsafe(8),
        exp: now_secs() + 60,
    };
    let token = sign_state("secret", &flow).unwrap();
    assert_eq!(verify_state("secret", &token).unwrap().code_verifier, verifier);
}

#[cfg(feature = "oauth")]
#[test]
fn oauth_profile_parse() {
    use sova_passport::oauth_test_support::parse_profile;
    use sova_passport::ProfileKind;
    use serde_json::json;
    let gh = parse_profile(
        ProfileKind::Github,
        &json!({"id": 42, "login": "ada", "email": "a@b.c"}),
    )
    .unwrap();
    assert_eq!(gh.provider_user_id, "42");
}

#[cfg(feature = "session")]
#[tokio::test]
async fn passport_login_logout_session() {
    use sova_passport::{Authenticated, Passport};
    use sova_session::memory_sessions;

    let mut app = App::new();
    app.install(memory_sessions());
    app.install(
        Passport::new().deserialize_user(|id, mut req| async move {
            req.set(User {
                id: id.parse().unwrap_or(0),
                name: format!("user-{id}"),
            });
            Ok(req)
        }),
    );
    app.get("/login", |mut req: Request| async move {
        req.login(
            "7",
            User {
                id: 7,
                name: "ada".into(),
            },
        );
        Response::text("ok")
    });
    app.get("/me", |req: Request| async move {
        assert!(req.is_authenticated());
        Ok::<_, sova_core::Error>(Json(req.require_user::<User>()?.clone()))
    });
    app.get("/logout", |mut req: Request| async move {
        req.logout();
        Response::text("bye")
    });

    let c = sova_core::TestClient::tracked(app).await.unwrap();
    c.get("/login").await;
    let me = c.get("/me").await;
    assert_eq!(me.status_code().as_u16(), 200);
    assert!(me
        .body_bytes()
        .map(|b| String::from_utf8_lossy(b).contains("user-7"))
        .unwrap_or(false));
    let _ = Authenticated { id: "7".into() };
}