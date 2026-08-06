//! Session requires CookieLayer; TestClient tracks sid cookies.

use ruvo_cookies::CookieLayer;
use ruvo_core::{App, Error, Html, Request, TestClient};
use ruvo_session::{memory_sessions, SessionExt};

#[tokio::test]
async fn session_sets_cookie_with_cookie_layer() {
    let mut app = App::new();
    app.install(CookieLayer);
    app.install(memory_sessions());
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Html("ok".to_string())
    });
    app.get("/read", |req: Request| async move {
        let v = req.session().get("k").unwrap_or_default();
        Html(v)
    });

    let c = TestClient::tracked(app).unwrap();
    let res = c.get("/").await;
    assert_eq!(res.status_code().as_u16(), 200);
    let set_cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .any(|v| v.contains("ruvo_sid="));
    assert!(set_cookie, "expected Set-Cookie from session layer");

    let read = c.get("/read").await;
    assert_eq!(read.body_bytes(), Some(b"v".as_slice()));
}

#[test]
fn session_without_cookie_layer_fails_build() {
    let mut app = App::new();
    app.install(memory_sessions());
    let err = match app.build() {
        Ok(_) => panic!("build must fail without cookies"),
        Err(err) => err,
    };
    assert!(matches!(err, Error::Internal(_)));
    assert!(
        err.to_string().contains("requires `cookies`"),
        "unexpected error: {err}"
    );
}
