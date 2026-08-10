//! Composite server + outbound client.

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};
use sova_core::{App, Request, ResponseAssert, TestClient};
use sova_graphql::{GraphQl, GraphQlExt};

struct Query;

#[Object]
impl Query {
    async fn ping(&self) -> &str {
        "pong"
    }
}

#[tokio::test]
async fn server_with_outbound_client_in_state() {
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
    let mut app = App::new();
    app.install(
        GraphQl::server(schema)
            .with_client("https://api.example.com/graphql")
            .graphiql(false)
            .without_subscriptions(),
    );
    app.get("/check", |req: Request| async move {
        let _ = req.try_graphql().expect("outbound client installed");
        sova_core::Json(serde_json::json!({ "ok": true }))
    });

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/graphql")
        .json(&serde_json::json!({ "query": "query { ping }" }))
        .await;
    res.assert_status(200);
    assert_eq!(res.json_value()["data"]["ping"], "pong");

    c.get("/check").await.assert_status(200);
}
