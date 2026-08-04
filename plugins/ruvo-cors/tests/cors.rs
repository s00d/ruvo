//! CORS plugin tests.

use http::Method;
use ruvo_core::{App, Plugin, Request, Response};
use ruvo_cors::Cors;

#[tokio::test]
async fn cors_preflight_has_acao() {
    let mut app = App::new();
    Cors::new().origin("*").install(&mut app);
    app.get("/api", |_r: Request| async { Response::text("ok") });

    let req = Request::builder()
        .method(Method::OPTIONS)
        .path("/api")
        .header("origin", "https://example.com")
        .header("access-control-request-method", "POST")
        .build();
    let res = app.handle(req).await;
    assert!(res.headers().get("access-control-allow-origin").is_some());
}
