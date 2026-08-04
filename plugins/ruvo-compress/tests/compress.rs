//! Compress plugin tests.

use bytes::Bytes;
use http::Method;
use http_body_util::{BodyExt, Full};
use ruvo_compress::Compress;
use ruvo_core::extend::BoxError;
use ruvo_core::{App, Plugin, Request, Response};
use std::convert::Infallible;

#[tokio::test]
async fn compress_collects_stream_body() {
    let mut app = App::new();
    Compress.install(&mut app);
    let big = "x".repeat(200);
    let big2 = big.clone();
    app.get("/", move |_r: Request| {
        let big2 = big2.clone();
        async move {
            let stream = Full::new(Bytes::from(big2))
                .map_err(|_: Infallible| -> BoxError { unreachable!() })
                .boxed();
            Response::stream(stream)
        }
    });

    let req = Request::builder()
        .method(Method::GET)
        .path("/")
        .header("accept-encoding", "gzip")
        .build();
    let res = app.handle(req).await;
    assert!(res.headers().get("content-encoding").is_some());
    let out = {
        let mut res = res;
        res.take_body().collect().await.unwrap()
    };
    assert_ne!(out.as_ref(), big.as_bytes());
    assert!(!out.is_empty());
}
