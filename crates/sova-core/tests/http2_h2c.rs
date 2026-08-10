#[allow(dead_code)]
mod common;

use bytes::Bytes;
use common::LiveServer;
use http::Method;
use http_body_util::BodyExt;
use http_body_util::Empty;
use hyper::Request as HyperRequest;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use sova_core::{App, Request, Response};

#[tokio::test]
async fn h2c_prior_knowledge_get_works() {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") });
    app.get("/forbidden", |_r: Request| async {
        Response::text("ok").header("Connection", "close")
    });

    let server = LiveServer::spawn(app).await;

    // Prior-knowledge h2c: no Upgrade/ALPN, client speaks HTTP/2 immediately.
    let client = Client::builder(TokioExecutor::new())
        .http2_only(true)
        .build_http();

    for (path, expect_status) in [("/", 200_u16), ("/forbidden", 500_u16)] {
        let uri = format!("http://{}{}", server.addr, path);
        let req = HyperRequest::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Empty::<Bytes>::new())
            .expect("request");

        let resp = client.request(req).await.expect("h2c request");
        assert_eq!(resp.status().as_u16(), expect_status);

        // Hop-by-hop forbidden headers must not leak back to clients on HTTP/2.
        assert!(
            resp.headers().get(http::header::CONNECTION).is_none(),
            "Connection header must be stripped on HTTP/2"
        );

        if expect_status == 200 {
            let body = resp.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(body.as_ref(), b"ok");
        }
    }

    server.shutdown().await;
}
