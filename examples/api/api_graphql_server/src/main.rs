//! GraphQL server demo — schema mount, GraphiQL, subscriptions, app state in resolvers.

use async_graphql::{Context, Object, Schema, Subscription};
use futures_util::Stream;
use sova::{App, GraphQl, GraphqlContext, Result};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

struct Query;

#[Object]
impl Query {
    async fn hello(&self) -> &str {
        "world"
    }

    async fn counter(&self, ctx: &Context<'_>) -> async_graphql::Result<i32> {
        let sova = ctx.data::<GraphqlContext>()?;
        Ok(sova
            .try_state::<Arc<AtomicI32>>()
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0))
    }
}

struct Mutation;

#[Object]
impl Mutation {
    async fn increment(&self, ctx: &Context<'_>) -> async_graphql::Result<i32> {
        let sova = ctx.data::<GraphqlContext>()?;
        let counter = sova.state::<Arc<AtomicI32>>();
        Ok(counter.fetch_add(1, Ordering::Relaxed) + 1)
    }
}

struct Live;

#[Subscription]
impl Live {
    async fn ticks(&self) -> impl Stream<Item = i32> {
        futures_util::stream::iter(0..3)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let counter = Arc::new(AtomicI32::new(0));
    let schema = Schema::build(Query, Mutation, Live).finish();

    let mut app = App::new();
    app.state(counter);
    app.install(
        GraphQl::server(schema)
            .path("/graphql")
            .graphiql_path("/graphiql")
            .graphiql(true)
            .subscriptions("/graphql/ws")
            .sdl_path("/graphql/sdl"),
    );

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    eprintln!("api_graphql_server http://127.0.0.1:{port}/graphiql");
    app.listen(port).await
}
