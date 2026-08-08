//! `[session]` unset-fill: cookie name → Set-Cookie.

use sova_core::{App, Html, Request, TestClient};
use sova_session::{memory_sessions, SessionExt};

#[tokio::test]
async fn session_cookie_name_from_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[session]
cookie = "my_sid"
"#,
    )
    .unwrap();
    app.install(memory_sessions());
    app.get("/", |req: Request| async move {
        req.session().set("k", "v");
        Html("ok".to_string())
    });

    let c = TestClient::tracked(app).unwrap();
    let res = c.get("/").await;
    assert_eq!(res.status_code().as_u16(), 200);
    let set_cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .any(|v| v.starts_with("my_sid=") || v.contains("my_sid="));
    assert!(set_cookie, "expected Set-Cookie my_sid=… from [session] cookie");
}
