//! Session API: regenerate, rolling, flash helpers, builder / toml edges.

use sova_core::{App, Html, Request, TestClient};
use sova_session::{
    memory_sessions, SameSite, SessionExt, SessionLayer, FLASH_ERRORS, FLASH_OLD, FLASH_STATUS,
};
use sova_store::{namespace, MemoryStore};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn regenerate_rotates_sid_keeps_data() {
    let store = Arc::new(namespace(Arc::new(MemoryStore::new()), "sess"));
    let mut app = App::new();
    app.install(SessionLayer::new(store).cookie_name("sid"));
    app.get("/in", |req: Request| async move {
        req.session().set("k", "v");
        Html(req.session().id())
    });
    app.get("/regen", |req: Request| async move {
        let before = req.session().id();
        req.session().regenerate();
        let after = req.session().id();
        assert_ne!(before, after);
        Html(format!("{}:{}", after, req.session().get("k").unwrap()))
    });
    app.get("/read", |req: Request| async move {
        Html(req.session().get("k").unwrap_or_else(|| "none".into()))
    });

    let c = TestClient::tracked(app).unwrap();
    let first = c.get("/in").await;
    let sid1 = String::from_utf8(first.body_bytes().unwrap().to_vec()).unwrap();
    let regen = c.get("/regen").await;
    let body = String::from_utf8(regen.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.ends_with(":v"));
    let sid2 = body.split(':').next().unwrap();
    assert_ne!(sid1, sid2);
    assert_eq!(c.get("/read").await.body_bytes(), Some(b"v".as_slice()));
}

#[tokio::test]
async fn rolling_refreshes_cookie_without_dirty() {
    let store = Arc::new(namespace(Arc::new(MemoryStore::new()), "sess"));
    let mut app = App::new();
    app.install(
        SessionLayer::new(store)
            .cookie_name("sid")
            .rolling(true)
            .save_uninitialized(true),
    );
    app.get("/", |_req: Request| async move { Html("ok".to_string()) });

    let c = TestClient::tracked(app).unwrap();
    let first = c.get("/").await;
    assert!(first
        .headers()
        .get_all("set-cookie")
        .iter()
        .any(|v| v.to_str().unwrap_or("").contains("sid=")));
    let second = c.get("/").await;
    assert!(second
        .headers()
        .get_all("set-cookie")
        .iter()
        .any(|v| v.to_str().unwrap_or("").contains("sid=")));
}

#[tokio::test]
async fn flash_helpers_and_session_mutations() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.get("/set", |req: Request| async move {
        let s = req.session();
        s.flash_status("Saved");
        s.flash_errors(&serde_json::json!({ "email": "bad" }));
        s.flash_old(&serde_json::json!({ "email": "a@b.c" }));
        s.set("keep", "1");
        s.get_or("missing", "fallback");
        Html("ok".to_string())
    });
    app.get("/take", |req: Request| async move {
        let s = req.session();
        assert_eq!(s.take(FLASH_STATUS), "Saved");
        assert_eq!(s.take_json(FLASH_ERRORS)["email"], "bad");
        assert_eq!(s.take_json(FLASH_OLD)["email"], "a@b.c");
        assert_eq!(s.get("keep").as_deref(), Some("1"));
        s.remove("keep");
        assert!(s.get("keep").is_none());
        s.set("a", "1");
        s.set("b", "2");
        s.clear();
        assert!(s.data().is_empty());
        let mut bag = HashMap::new();
        bag.insert("x".into(), "y".into());
        s.replace(bag);
        Html(s.get("x").unwrap())
    });
    app.get("/ext", |req: Request| async move {
        req.flash("k", "v");
        req.flash_status("hi");
        req.flash_errors(&serde_json::json!({}));
        req.flash_old(&serde_json::json!({}));
        Html(req.session().take("k"))
    });

    let c = TestClient::tracked(app).unwrap();
    c.get("/set").await;
    assert_eq!(c.get("/take").await.body_bytes(), Some(b"y".as_slice()));
    assert_eq!(c.get("/ext").await.body_bytes(), Some(b"v".as_slice()));
}

#[tokio::test]
async fn builder_flags_and_toml_ttl_same_site() {
    let store = Arc::new(namespace(Arc::new(MemoryStore::new()), "sess"));
    let mut app = App::new();
    app.configure_from_str(
        r#"
[session]
ttl = "2h"
same_site = "strict"
secure = true
"#,
    )
    .unwrap();
    app.install(
        SessionLayer::new(store)
            .http_only(true)
            .path("/app")
            .same_site(SameSite::Lax) // explicit wins over toml
            .ttl(Duration::from_secs(10)), // explicit wins
    );
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Html("ok".to_string())
    });

    let c = TestClient::tracked(app).unwrap();
    let res = c.get("/").await;
    let cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.contains("sova_sid="))
        .unwrap_or("")
        .to_string();
    assert!(cookie.contains("Path=/app"), "{cookie}");
    assert!(cookie.contains("HttpOnly"), "{cookie}");
    // Explicit SameSite::Lax (not Strict from toml).
    assert!(
        cookie.to_ascii_lowercase().contains("samesite=lax"),
        "{cookie}"
    );
}

#[tokio::test]
async fn toml_same_site_when_unset() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[session]
same_site = "none"
cookie = "s"
"#,
    )
    .unwrap();
    app.install(memory_sessions());
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Html("ok".to_string())
    });
    let c = TestClient::tracked(app).unwrap();
    let res = c.get("/").await;
    let cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.contains("s="))
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(cookie.contains("samesite=none"), "{cookie}");
}

#[tokio::test]
async fn empty_session_without_layer_is_local_only() {
    let mut app = App::new();
    app.get("/", |req: Request| async move {
        let s = req.session();
        s.set("k", "v");
        assert_eq!(s.get("k").as_deref(), Some("v"));
        assert!(s.user_id().is_none());
        Html("ok".to_string())
    });
    let c = TestClient::tracked(app).unwrap();
    assert_eq!(c.get("/").await.status_code().as_u16(), 200);
}

#[tokio::test]
async fn logout_without_bound_user_is_bad_request() {
    let mut app = App::new();
    app.install(memory_sessions());
    app.get("/x", |req: Request| async move {
        let err = req.logout_other_sessions().await.unwrap_err();
        Html(err.to_string())
    });
    let c = TestClient::tracked(app).unwrap();
    let res = c.get("/x").await;
    let body = String::from_utf8(res.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("no bound"), "{body}");
}
