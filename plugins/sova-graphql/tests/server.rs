//! Schema mount + GraphiQL smoke tests (feature `server`).

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};
use sova_core::{App, ResponseAssert, TestClient};
use sova_graphql::GraphQl;

struct Query;

#[Object]
impl Query {
    async fn hello(&self) -> &str {
        "world"
    }
}

#[tokio::test]
async fn server_post_query() {
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
    let mut app = App::new();
    app.install(
        GraphQl::server(schema)
            .path("/graphql")
            .graphiql(false)
            .without_subscriptions(),
    );

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/graphql")
        .json(&serde_json::json!({ "query": "query { hello }" }))
        .await;
    res.assert_status(200);
    assert_eq!(res.json_value()["data"]["hello"], "world");
}

#[tokio::test]
async fn server_graphiql_on_separate_path() {
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
    let mut app = App::new();
    app.install(
        GraphQl::server(schema)
            .path("/graphql")
            .graphiql_path("/graphiql")
            .graphiql(true)
            .without_subscriptions(),
    );

    let c = TestClient::new(app).unwrap();
    let api = c.get("/graphql").await;
    api.assert_status(405);

    let ui = c.get("/graphiql").await;
    ui.assert_status(200);
    let body = String::from_utf8_lossy(ui.body_bytes().unwrap());
    assert!(body.contains("GraphiQL") || body.contains("graphiql"));
}

#[tokio::test]
async fn server_get_query_when_enabled() {
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
    let mut app = App::new();
    app.install(
        GraphQl::server(schema)
            .path("/graphql")
            .graphiql(false)
            .allow_get_queries(true)
            .without_subscriptions(),
    );

    let c = TestClient::new(app).unwrap();
    let res = c.get("/graphql?query=query%20%7B%20hello%20%7D").await;
    res.assert_status(200);
    assert_eq!(res.json_value()["data"]["hello"], "world");
}

#[tokio::test]
async fn server_sdl_endpoint() {
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
    let mut app = App::new();
    app.install(
        GraphQl::server(schema)
            .graphiql(false)
            .sdl_path("/graphql/sdl")
            .without_subscriptions(),
    );

    let c = TestClient::new(app).unwrap();
    let res = c.get("/graphql/sdl").await;
    res.assert_status(200);
    let body = String::from_utf8_lossy(res.body_bytes().unwrap());
    assert!(body.contains("hello"));
}
