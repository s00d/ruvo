//! Session installs CookieLayer automatically.

use http::Method;
use ruvo_core::{App, Html, Plugin, Request};
use ruvo_session::{memory_sessions, SessionExt};

#[tokio::test]
async fn session_sets_cookie_without_manual_cookie_layer() {
    let mut app = App::new();
    memory_sessions().install(&mut app);
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Html("ok".to_string())
    });

    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(res.status_code().as_u16(), 200);
    let set_cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .any(|v| v.contains("ruvo_sid="));
    assert!(set_cookie, "expected Set-Cookie from session layer");
}
