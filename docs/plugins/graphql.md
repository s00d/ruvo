---
title: graphql
editLink: false
---

# `graphql`

**Outbound GraphQL client (+ optional schema mount)**

| | |
|--|--|
| Crate | [`sova-graphql`](https://docs.rs/sova-graphql/0.1.0) `0.1.0` |
| Plugin id | `graphql` |
| Category | Integrations |

## Install

```bash
cargo add sova --features graphql
```

## Features

| Feature | What you get |
|---------|-------------|
| `graphql` | Outbound GraphQL client (`req.graphql()`, FakeGraphql). |
| `graphql-server` | `graphql` + async-graphql schema mount / GraphiQL. |

## Overview

GraphQL for Sova — **client first**, optional schema mount.

```rust
 use sova_graphql::{FakeGraphql, GraphQl, GraphQlExt};
 use serde_json::json;

 let fake = FakeGraphql::new().stub("hello", json!({ "hello": "world" }));
 app.install(GraphQl::fake(fake.clone()));

 // in a handler:
 let v = req.graphql().query("query { hello }").await?;
 assert_eq!(v["hello"], "world");
 ```

## Quick start

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

## Examples

- [`examples/api/api_graphql`](https://github.com/s00d/sova/tree/master/examples/api/api_graphql)

## Related

[`http`](/plugins/http) · [`grpc`](/plugins/grpc)
