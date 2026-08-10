//! Identity keys + login throttle presets.

use http::Method;
use serde_json::json;
use sova_core::{App, Plugin, RateLimitIdentity, Request, Response, Router, TestClient};
use sova_rate_limit::{RateLimit, RateLimitKey};
use std::time::Duration;

#[tokio::test]
async fn identity_keys_are_isolated() {
    let mut app = App::new();
    RateLimit::new(2, Duration::from_secs(60))
        .key(RateLimitKey::Identity)
        .install(&mut app);
    app.get("/", |_r: Request| async { Response::text("ok") });

    let server = app.build().unwrap();

    for _ in 0..2 {
        let mut req = Request::builder().method(Method::GET).path("/").build();
        req.set(RateLimitIdentity("user-a".into()));
        let res = server.handle(req).await;
        assert_eq!(res.status_code().as_u16(), 200);
    }
    {
        let mut req = Request::builder().method(Method::GET).path("/").build();
        req.set(RateLimitIdentity("user-a".into()));
        let res = server.handle(req).await;
        assert_eq!(res.status_code().as_u16(), 429);
    }
    // Different identity still allowed.
    {
        let mut req = Request::builder().method(Method::GET).path("/").build();
        req.set(RateLimitIdentity("user-b".into()));
        let res = server.handle(req).await;
        assert_eq!(res.status_code().as_u16(), 200);
    }
}

#[tokio::test]
async fn login_preset_keys_by_email() {
    let mut app = App::new();
    let mut r = Router::new();
    r.post("/", |_r: Request| async { Response::text("ok") });
    r.route_middleware(RateLimit::login().middleware());
    app.mount("/login", r);

    let c = TestClient::tracked(app).await.unwrap();

    for _ in 0..5 {
        let res = c
            .post("/login")
            .header("content-type", "application/json")
            .json(&json!({ "email": "ada@example.com", "password": "x" }))
            .await;
        assert_eq!(
            res.status_code().as_u16(),
            200,
            "body={}",
            String::from_utf8_lossy(res.body_bytes().unwrap_or_default())
        );
    }
    let blocked = c
        .post("/login")
        .header("content-type", "application/json")
        .json(&json!({ "email": "ada@example.com", "password": "x" }))
        .await;
    assert_eq!(blocked.status_code().as_u16(), 429);

    // Other email still ok (same IP).
    let other = c
        .post("/login")
        .header("content-type", "application/json")
        .json(&json!({ "email": "bob@example.com", "password": "x" }))
        .await;
    assert_eq!(other.status_code().as_u16(), 200);
}
