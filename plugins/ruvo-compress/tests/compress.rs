//! Compress plugin tests.

use bytes::Bytes;
use http::Method;
use http_body_util::{BodyExt, Full};
use ruvo_compress::Compress;
use ruvo_core::extend::BoxError;
use ruvo_core::{App, Plugin, Request, Response};
use std::convert::Infallible;

fn app_with(compress: Compress) -> App {
    let mut app = App::new();
    compress.install(&mut app);
    app
}

#[tokio::test]
async fn compress_gzip_stream_body() {
    let mut app = app_with(Compress::new().threshold(100));
    let big = "x".repeat(200);
    let big2 = big.clone();
    app.get("/", move |_r: Request| {
        let big2 = big2.clone();
        async move {
            let stream = Full::new(Bytes::from(big2))
                .map_err(|_: Infallible| -> BoxError { unreachable!() })
                .boxed();
            Response::stream(stream).header("content-type", "text/plain")
        }
    });

    let req = Request::builder()
        .method(Method::GET)
        .path("/")
        .header("accept-encoding", "gzip")
        .build();
    let res = app.handle(req).await;
    assert_eq!(
        res.headers().get("content-encoding").and_then(|v| v.to_str().ok()),
        Some("gzip")
    );
    assert!(res
        .headers()
        .get("vary")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .contains("Accept-Encoding"));
    let out = {
        let mut res = res;
        res.take_body().collect().await.unwrap()
    };
    assert_ne!(out.as_ref(), big.as_bytes());
    assert!(!out.is_empty());
}

#[tokio::test]
async fn prefers_brotli_over_gzip() {
    let mut app = app_with(Compress::new().threshold(10));
    app.get("/", |_r: Request| async {
        Response::text("y".repeat(64)).header("content-type", "text/plain")
    });
    let req = Request::builder()
        .method(Method::GET)
        .path("/")
        .header("accept-encoding", "gzip, br")
        .build();
    let res = app.handle(req).await;
    assert_eq!(
        res.headers().get("content-encoding").and_then(|v| v.to_str().ok()),
        Some("br")
    );
}

#[tokio::test]
async fn respects_q_values() {
    let mut app = app_with(Compress::new().threshold(10));
    app.get("/", |_r: Request| async {
        Response::text("z".repeat(64)).header("content-type", "text/plain")
    });
    let req = Request::builder()
        .method(Method::GET)
        .path("/")
        .header("accept-encoding", "br;q=0, gzip;q=1.0")
        .build();
    let res = app.handle(req).await;
    assert_eq!(
        res.headers().get("content-encoding").and_then(|v| v.to_str().ok()),
        Some("gzip")
    );
}

#[tokio::test]
async fn skips_below_threshold() {
    let mut app = app_with(Compress::new().threshold(10_000));
    app.get("/", |_r: Request| async {
        Response::text("tiny").header("content-type", "text/plain")
    });
    let req = Request::builder()
        .method(Method::GET)
        .path("/")
        .header("accept-encoding", "gzip")
        .build();
    let res = app.handle(req).await;
    assert!(res.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn skips_image_and_x_no_compression() {
    let mut app = app_with(Compress::new().threshold(1));
    app.get("/img", |_r: Request| async {
        Response::text("i".repeat(64)).header("content-type", "image/png")
    });
    app.get("/plain", |_r: Request| async {
        Response::text("p".repeat(64)).header("content-type", "text/plain")
    });

    let img = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/img")
                .header("accept-encoding", "gzip")
                .build(),
        )
        .await;
    assert!(img.headers().get("content-encoding").is_none());

    let skip = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/plain")
                .header("accept-encoding", "gzip")
                .header("x-no-compression", "1")
                .build(),
        )
        .await;
    assert!(skip.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn skips_cache_control_no_transform() {
    let mut app = app_with(Compress::new().threshold(1));
    app.get("/", |_r: Request| async {
        Response::text("n".repeat(64))
            .header("content-type", "text/plain")
            .header("cache-control", "no-transform")
    });
    let res = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("accept-encoding", "gzip")
                .build(),
        )
        .await;
    assert!(res.headers().get("content-encoding").is_none());
}

#[tokio::test]
async fn deflate_encoding() {
    let mut app = app_with(Compress::new().threshold(1));
    app.get("/", |_r: Request| async {
        Response::text("d".repeat(64)).header("content-type", "application/json")
    });
    let res = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("accept-encoding", "deflate")
                .build(),
        )
        .await;
    assert_eq!(
        res.headers().get("content-encoding").and_then(|v| v.to_str().ok()),
        Some("deflate")
    );
}

#[tokio::test]
async fn weakens_strong_etag() {
    let mut app = app_with(Compress::new().threshold(1));
    app.get("/", |_r: Request| async {
        Response::text("e".repeat(64))
            .header("content-type", "text/plain")
            .header("etag", "\"abc\"")
    });
    let res = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("accept-encoding", "gzip")
                .build(),
        )
        .await;
    assert_eq!(
        res.headers().get("etag").and_then(|v| v.to_str().ok()),
        Some("W/\"abc\"")
    );
}
