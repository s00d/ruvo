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

#[tokio::test]
async fn facade_default_from_and_bound_builders() {
    let mut app = App::default();
    app.get("/z", |_r: Request| async { Response::text("z") });
    let core: ruvo_core::App = app.into();
    let app2 = App::from(core);
    assert_eq!(
        app2.handle_request(Method::GET, "/z", "")
            .await
            .body_bytes(),
        Some(b"z".as_slice())
    );

    let bound = App::new()
        .bind("127.0.0.1:0")
        .http(ruvo::Http::H1)
        .reuseport(false)
        .shutdown(async {});
    // Drop without serve — builders covered.
    drop(bound);
}
