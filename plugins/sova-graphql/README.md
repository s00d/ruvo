[![crates.io](https://img.shields.io/crates/v/sova-graphql?style=for-the-badge)](https://crates.io/crates/sova-graphql)
[![docs.rs](https://img.shields.io/docsrs/sova-graphql?style=for-the-badge)](https://docs.rs/sova-graphql)
[![License](https://img.shields.io/crates/l/sova-graphql?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)

# sova-graphql

GraphQL for Sova: outbound HTTP client (`req.graphql()`), optional **schema server** (GraphiQL, subscriptions, `GraphqlContext`).

Part of [Sova](https://crates.io/crates/sova).

**Guide:** [https://s00d.github.io/sova/plugins/graphql](https://s00d.github.io/sova/plugins/graphql)

## Features

| Feature | Description |
|---------|-------------|
| `client` (default) | Outbound HTTP client + `FakeGraphql` |
| `server` | Schema mount, GraphiQL, WebSocket subscriptions, resolver context |

## Install

```bash
# Server + client
cargo add sova --features graphql-server

# Client only
cargo add sova --features graphql
```

## Server (3 steps)

```rust
use async_graphql::{Object, Schema};
use sova::GraphQl;

#[Object]
impl Query { async fn hello(&self) -> &str { "world" } }

let schema = Schema::build(Query, async_graphql::EmptyMutation, async_graphql::EmptySubscription).finish();
app.install(GraphQl::server(schema).graphiql(true));
```

Resolvers: `ctx.data::<sova::GraphqlContext>()?.state::<YourState>()`.

## License

MIT — see [LICENSE](LICENSE).
