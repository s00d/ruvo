//! Resolver access to Sova app state via [`GraphqlContext`].

use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema};
use sova_core::{App, ResponseAssert, TestClient};
use sova_graphql::{GraphQl, GraphqlContext};

struct Query;

#[Object]
impl Query {
    async fn counter(&self, ctx: &Context<'_>) -> async_graphql::Result<i32> {
        let sova = ctx.data::<GraphqlContext>()?;
        Ok(sova.try_state::<i32>().map(|v| *v).unwrap_or(0))
    }
}

#[tokio::test]
async fn resolver_reads_app_state() {
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
    let mut app = App::new();
    app.state(7i32);
    app.install(
        GraphQl::server(schema)
            .path("/graphql")
            .graphiql(false)
            .without_subscriptions(),
    );

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/graphql")
        .json(&serde_json::json!({ "query": "query { counter }" }))
        .await;
    res.assert_status(200);
    assert_eq!(res.json_value()["data"]["counter"], 7);
}
