//! Client-focused GraphQL smoke tests.

use sova_core::{App, Request, ResponseAssert, TestClient};
use sova_graphql::{FakeGraphql, GraphQl, GraphQlExt};
use serde_json::json;

#[tokio::test]
async fn fake_client_query() {
    let fake = FakeGraphql::new().stub("hello", json!({ "hello": "world" }));
    let mut app = App::new();
    app.install(GraphQl::fake(fake.clone()));
    app.get("/ping", |req: Request| async move {
        let data = req.graphql().query("query { hello }").data().await.unwrap();
        sova_core::Json(data)
    });

    let c = TestClient::new(app).unwrap();
    let res = c.get("/ping").await;
    res.assert_status(200);
    assert_eq!(res.json_value()["hello"], "world");
    fake.assert_called_with("hello");
}
