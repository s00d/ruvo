//! Observability metrics smoke.

use sova_core::{request_id, App, Html, Request, TestClient};
use sova_observability::Observability;

#[tokio::test]
async fn metrics_endpoint_and_labels() {
    let mut app = App::new();
    app.use_middleware(request_id());
    app.install(Observability::new());
    app.get("/hello/:name", |req: Request| async move {
        let name = req.param("name").unwrap_or("?");
        Html(format!("hi {name}"))
    });

    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/hello/world").await;
    assert_eq!(res.status_code().as_u16(), 200);
    assert!(res.headers().get("x-request-id").is_some());

    let metrics = c.get("/metrics").await;
    assert_eq!(metrics.status_code().as_u16(), 200);
    let body = String::from_utf8(metrics.body_bytes().unwrap().to_vec()).unwrap();
    assert!(
        body.contains("http_requests_total"),
        "missing counter: {body}"
    );
    assert!(
        body.contains("/hello/:name") || body.contains("hello"),
        "missing route label: {body}"
    );
}
