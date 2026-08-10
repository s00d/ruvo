**Audience:** app authors calling remote GraphQL APIs (and optionally mounting a schema).

Outbound client is the primary surface. Schema mount needs feature `server` / facade `graphql-server`.

## Client

```toml
[dev-dependencies]
sova = { version = "0.1", features = ["graphql", "testing"] }
```

```rust
use sova::{FakeGraphql, GraphQl, GraphQlExt};
use serde_json::json;

let fake = FakeGraphql::new().stub("hello", json!({ "hello": "world" }));
app.install(GraphQl::fake(fake));

// handler:
let data = req.graphql().query("query { hello }").data().await?;
```

Live endpoint: `GraphQl::client("https://api.example.com/graphql")` or `[graphql] url=…` / `GRAPHQL_URL`.

## Server (optional)

```toml
sova = { features = ["graphql-server"] }
```

```rust
use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};
use sova::GraphQl;

struct Query;
#[Object]
impl Query {
    async fn hello(&self) -> &str { "world" }
}

let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
app.install(GraphQl::server(schema).path("/graphql").graphiql(true));
```
