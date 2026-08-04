use futures_util::stream;
use futures_util::FutureExt;
use http::Method;
use ruvo_core::extend::IntoHandler;
use ruvo_core::{App, Error, Html, Json, Request, Response};
use std::convert::Infallible;
use std::panic::AssertUnwindSafe;

#[tokio::test]
async fn error_handler_catches_handler_err() {
    let mut app = App::new();
    app.error_handler(|err| async move {
        Response::text(format!("custom:{err}")).status(418)
    });
    app.get(
        "/deny",
        (|_r: Request| async { Err::<Response, Error>(Error::Unauthorized) }).into_handler(),
    );
    let res = app.handle_request(Method::GET, "/deny", "").await;
    assert_eq!(res.status_code().as_u16(), 418);
    assert_eq!(
        res.body_bytes(),
        Some(b"custom:unauthorized".as_slice())
    );
}

#[tokio::test]
async fn html_json_handler_returns() {
    let mut app = App::new();
    app.get("/h", |_r: Request| async { Html("<p>hi</p>".to_string()) });
    app.get("/j", |_r: Request| async { Json(serde_json::json!({"n": 1})) });

    let h = app.handle_request(Method::GET, "/h", "").await;
    assert_eq!(h.body_bytes(), Some(b"<p>hi</p>".as_slice()));
    let j = app.handle_request(Method::GET, "/j", "").await;
    assert!(j.body_bytes().unwrap().windows(1).any(|w| w == b"1"));
}

#[tokio::test]
async fn sse_streams_events() {
    let res = Response::sse(stream::iter(vec![
        Ok::<_, Infallible>("one".into()),
        Ok("two\nlines".into()),
    ]));
    assert_eq!(
        res.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );
    let mut res = res;
    let bytes = res.take_body().collect().await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("data: one\n\n"));
    assert!(text.contains("data: two\n"));
    assert!(text.contains("data: lines\n"));
}

#[tokio::test]
async fn panic_in_handler_becomes_500_via_unwind() {
    let mut app = App::new();
    app.get("/boom", |_r: Request| async {
        panic!("boom");
        #[allow(unreachable_code)]
        Ok::<Response, Error>(Response::text("nope"))
    });

    let res = match AssertUnwindSafe(app.handle_request(Method::GET, "/boom", ""))
        .catch_unwind()
        .await
    {
        Ok(res) => res,
        Err(_) => Response::text("Internal Server Error").status(500),
    };
    assert_eq!(res.status_code().as_u16(), 500);
}
