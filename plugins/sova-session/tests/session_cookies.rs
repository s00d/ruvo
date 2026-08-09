//! Session auto-installs CookieLayer when missing; TestClient tracks sid cookies.

use sova_cookies::CookieLayer;
use sova_core::{App, Html, Request, TestClient};
use sova_session::{memory_sessions, SessionExt};

#[tokio::test]
async fn session_sets_cookie_with_cookie_layer() {
    let mut app = App::new();
    app.install(CookieLayer);
    app.install(memory_sessions());
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Html("ok".to_string())
    });
    app.get("/read", |req: Request| async move {
        let v = req.session().get("k").unwrap_or_default();
        Html(v)
    });

    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/").await;
    assert_eq!(res.status_code().as_u16(), 200);
    let set_cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .any(|v| v.contains("sova_sid="));
    assert!(set_cookie, "expected Set-Cookie from session layer");

    let read = c.get("/read").await;
    assert_eq!(read.body_bytes(), Some(b"v".as_slice()));
}

#[tokio::test]
async fn production_env_enables_secure_cookie() {
    let prev_env = std::env::var("SOVA_ENV").ok();
    let prev_secure = std::env::var("SESSION_SECURE").ok();
    std::env::remove_var("SESSION_SECURE");
    std::env::set_var("SOVA_ENV", "production");

    let mut app = App::new();
    app.install(memory_sessions());
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Html("ok".to_string())
    });
    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/").await;
    let cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.contains("sova_sid="))
        .expect("sid cookie");
    assert!(
        cookie.to_ascii_lowercase().contains("secure"),
        "expected Secure in production: {cookie}"
    );

    match prev_env {
        Some(v) => std::env::set_var("SOVA_ENV", v),
        None => std::env::remove_var("SOVA_ENV"),
    }
    match prev_secure {
        Some(v) => std::env::set_var("SESSION_SECURE", v),
        None => std::env::remove_var("SESSION_SECURE"),
    }
}

#[tokio::test]
async fn session_auto_installs_cookie_layer() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Html("ok".to_string())
    });
    app.get("/read", |req: Request| async move {
        let v = req.session().get("k").unwrap_or_default();
        Html(v)
    });

    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/").await;
    assert_eq!(res.status_code().as_u16(), 200);
    let read = c.get("/read").await;
    assert_eq!(read.body_bytes(), Some(b"v".as_slice()));
}

#[tokio::test]
async fn save_uninitialized_false_skips_cookie() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.get("/", |_req: Request| async move { Html("ok".to_string()) });

    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/").await;
    let set_cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .any(|v| v.contains("sova_sid="));
    assert!(!set_cookie, "empty session must not Set-Cookie");
}

#[tokio::test]
async fn destroy_clears_session() {
    use sova_session::SessionLayer;
    use sova_store::{namespace, MemoryStore};
    use std::sync::Arc;

    let store = Arc::new(namespace(Arc::new(MemoryStore::new()), "sess"));
    let mut app = App::new();
    app.install(SessionLayer::new(store).cookie_name("sid"));
    app.get("/in", |req: Request| async move {
        req.session().set("k", "v");
        Html("ok".to_string())
    });
    app.get("/out", |req: Request| async move {
        req.session().destroy();
        Html("bye".to_string())
    });
    app.get("/read", |req: Request| async move {
        Html(req.session().get("k").unwrap_or_else(|| "none".into()))
    });

    let c = TestClient::tracked(app).await.unwrap();
    c.get("/in").await;
    assert_eq!(c.get("/read").await.body_bytes(), Some(b"v".as_slice()));
    c.get("/out").await;
    assert_eq!(c.get("/read").await.body_bytes(), Some(b"none".as_slice()));
}

#[tokio::test]
async fn hook_hydrates_request() {
    use sova_session::SessionLayer;
    use sova_store::{namespace, MemoryStore};
    use std::sync::Arc;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct CurrentUser(String);

    let store = Arc::new(namespace(Arc::new(MemoryStore::new()), "sess"));
    let mut app = App::new();
    app.install(
        SessionLayer::new(store).hook(|sess, mut req| async move {
            if let Some(name) = sess.get("user") {
                req.set(CurrentUser(name));
            }
            Ok(req)
        }),
    );
    app.get("/login", |req: Request| async move {
        req.session().set("user", "ada");
        Html("ok".to_string())
    });
    app.get("/me", |req: Request| async move {
        let name = req
            .get::<CurrentUser>()
            .map(|u| u.0.clone())
            .unwrap_or_else(|| "anon".into());
        Html(name)
    });

    let c = TestClient::tracked(app).await.unwrap();
    assert_eq!(c.get("/me").await.body_bytes(), Some(b"anon".as_slice()));
    c.get("/login").await;
    assert_eq!(c.get("/me").await.body_bytes(), Some(b"ada".as_slice()));
}
