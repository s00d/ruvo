//! Facade glue tests — plugin coverage lives in each plugin crate.

use ruvo::{App, Request, Response};
use http::Method;

#[tokio::test]
async fn facade_reexports_core_handle() {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(res.body_bytes(), Some(b"ok".as_slice()));
}
