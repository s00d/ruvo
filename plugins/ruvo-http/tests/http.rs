use http::Method;
use ruvo_core::extend::Deadline;
use ruvo_core::{App, IntoResponse, Request, Response};
use ruvo_http::{FakeTransport, Http, HttpError, HttpExt, StubBody};
use serde_json::json;
use std::time::{Duration, Instant};

#[tokio::test]
async fn fake_get_and_assert() {
    let fake = FakeTransport::new().get(
        "https://api.example.com/users/1",
        StubBody::Json(json!({ "id": 1 })),
    );
    let mut app = App::new();
    app.install(Http::new().with_fake(fake.clone()));
    app.get("/proxy", |req: Request| async move {
        let res = req
            .http()
            .get("https://api.example.com/users/1")
            .send()
            .await
            .unwrap();
        Response::json(&res.json::<serde_json::Value>().unwrap())
    });
    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/proxy", "").await;
    assert_eq!(res.status_code().as_u16(), 200);
    fake.assert_sent(Method::GET, "https://api.example.com/users/1");
    fake.assert_sent_count(1);
}

#[tokio::test]
async fn budget_caps_timeout() {
    let fake = FakeTransport::new().get("https://api.example.com/x", StubBody::Empty);
    let mut app = App::new();
    app.install(Http::new().with_fake(fake.clone()));
    app.get("/t", |req: Request| async move {
        let _ = req.http().get("https://api.example.com/x").send().await;
        Response::text("ok")
    });
    let server = app.build().unwrap();
    let mut req = Request::builder()
        .method(Method::GET)
        .path("/t")
        .build();
    req.set(Deadline(Instant::now() + Duration::from_millis(50)));
    let _ = server.handle(req).await;
    let sent = fake.sent();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].timeout.unwrap() <= Duration::from_millis(50));
}

#[tokio::test]
async fn propagates_request_id() {
    let fake = FakeTransport::new().get("https://api.example.com/x", StubBody::Empty);
    let mut app = App::new();
    app.install(Http::new().with_fake(fake.clone()));
    app.get("/t", |req: Request| async move {
        let _ = req.http().get("https://api.example.com/x").send().await;
        Response::text("ok")
    });
    let server = app.build().unwrap();
    let req = Request::builder()
        .method(Method::GET)
        .path("/t")
        .header("x-request-id", "abc-123")
        .build();
    let _ = server.handle(req).await;
    let sent = fake.sent();
    assert_eq!(
        sent[0]
            .headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok()),
        Some("abc-123")
    );
}

#[tokio::test]
async fn four_xx_is_ok_error_for_status_errs() {
    let fake = FakeTransport::new().respond(
        Method::GET,
        "https://api.example.com/missing",
        404,
        StubBody::Empty,
    );
    let mut app = App::new();
    app.install(Http::new().with_fake(fake));
    app.get("/x", |req: Request| async move {
        let res = req
            .http()
            .get("https://api.example.com/missing")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status_u16(), 404);
        match res.error_for_status() {
            Err(HttpError::Status(404)) => Response::text("mapped"),
            _ => Response::text("bad"),
        }
    });
    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/x", "").await;
    assert_eq!(res.body_bytes(), Some(b"mapped".as_slice()));
}

#[tokio::test]
async fn post_without_idempotency_does_not_retry() {
    let fake = FakeTransport::new().fail("https://api.example.com/pay", "timeout");
    let mut app = App::new();
    app.install(Http::new().with_fake(fake.clone()));
    app.post("/x", |req: Request| async move {
        let err = req
            .http()
            .post("https://api.example.com/pay")
            .retry(3)
            .send()
            .await
            .unwrap_err();
        assert!(matches!(err, HttpError::Timeout));
        Response::text("ok")
    });
    let server = app.build().unwrap();
    let _ = server
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/x")
                .build(),
        )
        .await;
    fake.assert_sent_count(1);
}

#[tokio::test]
async fn get_retries_on_timeout() {
    let fake = FakeTransport::new().fail("https://api.example.com/x", "timeout");
    let mut app = App::new();
    app.install(Http::new().with_fake(fake.clone()));
    app.get("/x", |req: Request| async move {
        let _ = req
            .http()
            .get("https://api.example.com/x")
            .retry(2)
            .send()
            .await;
        Response::text("ok")
    });
    let server = app.build().unwrap();
    let _ = server.handle_request(Method::GET, "/x", "").await;
    fake.assert_sent_count(3);
}

#[tokio::test]
async fn named_client_joins_base_url() {
    let fake = FakeTransport::new().post("https://api.stripe.com/v1/charges", 201);
    let mut app = App::new();
    app.configure_from_str(
        r#"
[default.http.payments]
base_url = "https://api.stripe.com"
"#,
    )
    .unwrap();
    app.install(Http::new().with_fake(fake.clone()));
    app.post("/pay", |req: Request| async move {
        let res = req
            .http()
            .named("payments")
            .post("/v1/charges")
            .send()
            .await
            .unwrap();
        Response::text(res.status_u16().to_string())
    });
    let server = app.build().unwrap();
    let res = server
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/pay")
                .build(),
        )
        .await;
    assert_eq!(res.body_bytes(), Some(b"201".as_slice()));
    fake.assert_sent(Method::POST, "https://api.stripe.com/v1/charges");
}

#[tokio::test]
async fn http_error_into_response_status() {
    assert_eq!(HttpError::Timeout.into_response().status_code().as_u16(), 504);
    assert_eq!(
        HttpError::Connect("x".into())
            .into_response()
            .status_code()
            .as_u16(),
        502
    );
}
