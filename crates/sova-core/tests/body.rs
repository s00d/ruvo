use http::Method;
use http_body_util::{BodyExt, Full};
use sova_core::extend::{Body, BoxError};
use sova_core::{App, Error, Request, Response};
use std::convert::Infallible;

#[tokio::test]
async fn query_plus_is_space() {
    let mut app = App::new();
    app.get("/q", |req: Request| async move {
        Response::text(req.query("q").unwrap_or("").to_string())
    });
    let res = app
        .handle_request(Method::GET, "/q?q=hello+world", "")
        .await;
    assert_eq!(res.body_bytes(), Some(b"hello world".as_slice()));
}

#[tokio::test]
async fn form_parse() {
    let mut app = App::new();
    app.post("/f", |mut req: Request| async move {
        #[derive(serde::Deserialize)]
        struct Form {
            user: String,
        }
        let f: Form = req.form().await.unwrap();
        Response::text(f.user)
    });
    let res = app.handle_request(Method::POST, "/f", "user=ada").await;
    assert_eq!(res.body_bytes(), Some(b"ada".as_slice()));
}

#[tokio::test]
async fn query_as_and_param_as() {
    let mut app = App::new();
    app.get("/u/:id", |req: Request| async move {
        #[derive(serde::Deserialize)]
        struct Q {
            q: String,
        }
        let id: i64 = req.param_as("id").unwrap();
        let q: Q = req.query_as().unwrap();
        Response::text(format!("{id}:{}", q.q))
    });
    let res = app.handle_request(Method::GET, "/u/7?q=hi", "").await;
    assert_eq!(res.body_bytes(), Some(b"7:hi".as_slice()));
}

#[tokio::test]
async fn body_consumed_names_consumer() {
    let mut req = Request::builder().body("hi").build();
    let _ = req.into_body_stream_as("multipart").unwrap();
    let err = req.json::<serde_json::Value>().await.unwrap_err();
    assert!(err.to_string().contains("multipart"), "{err}");
}

#[tokio::test]
async fn take_body_collect_bytes_and_stream() {
    let mut res = Response::text("hello");
    let bytes = res.take_body().collect().await.unwrap();
    assert_eq!(bytes.as_ref(), b"hello");

    let stream = Full::new(bytes::Bytes::from_static(b"streamed"))
        .map_err(|_: Infallible| -> BoxError { unreachable!() })
        .boxed();
    let body = Body::Stream(stream);
    let collected = body.collect().await.unwrap();
    assert_eq!(collected.as_ref(), b"streamed");
}

#[tokio::test]
async fn payload_too_large_maps_413() {
    let res = Error::PayloadTooLarge.into_response();
    assert_eq!(res.status_code().as_u16(), 413);
}

#[tokio::test]
async fn max_body_route_overrides_app() {
    use sova_core::extend::MaxBody;

    let mut app = App::new();
    app.max_body_size(10);
    app.post("/small", |mut req: Request| async move {
        let _ = req.body().await?;
        Ok::<_, Error>(Response::text("ok"))
    });
    app.post("/upload", |mut req: Request| async move {
        let b = req.body().await?;
        Ok::<_, Error>(Response::text(format!("{}", b.len())))
    })
    .with(MaxBody::bytes(100));

    let big = "x".repeat(50);
    let deny = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/small")
                .body(big.clone())
                .body_limit(10)
                .build(),
        )
        .await;
    assert_eq!(deny.status_code().as_u16(), 413);

    let allow = app
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/upload")
                .body(big)
                .body_limit(10)
                .build(),
        )
        .await;
    assert_eq!(allow.status_code().as_u16(), 200);
    assert_eq!(allow.body_bytes(), Some(b"50".as_slice()));
}
