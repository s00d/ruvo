---
title: graphql
editLink: false
---

# `graphql`

**Outbound GraphQL client (+ optional schema mount)**

| | |
|--|--|
| Crate | [`sova-graphql`](https://docs.rs/sova-graphql/0.1.2) `0.1.2` |
| Plugin id | `graphql` |
| Category | APIs |

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

GraphQL for Sova — client + optional schema server mount.

```rust
 use sova_graphql::{FakeGraphql, GraphQl, GraphQlExt};
 use serde_json::json;

 let fake = FakeGraphql::new().stub("hello", json!({ "hello": "world" }));
 app.install(GraphQl::fake(fake.clone()));

 // in a handler:
 let v = req.graphql().query("query { hello }").data().await?;
 assert_eq!(v["hello"], "world");
 ```

## Quick start

**Audience:** app authors mounting a GraphQL API on Sova (and optionally calling remote APIs).

Facade: `graphql-server` = outbound client features + schema mount + GraphiQL + WebSocket subscriptions.

## Quick start: GraphQL server

```toml
sova = { version = "0.1", features = ["graphql-server"] }
```

```rust
use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema};
use sova::{App, GraphQl, GraphqlContext};

struct Query;
#[Object]
impl Query {
    async fn hello(&self) -> &str { "world" }

    async fn value(&self, ctx: &Context<'_>) -> async_graphql::Result<i32> {
        let sova = ctx.data::<GraphqlContext>()?;
        Ok(*sova.state::<i32>())
    }
}

let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
app.install(
    GraphQl::server(schema)
        .path("/graphql")
        .graphiql_path("/graphiql")
        .graphiql(true)
        .subscriptions("/graphql/ws"),
);
```

Open `GET /graphiql` in the browser. API lives at `POST /graphql`.

## Resolvers + app state

Every request injects [`GraphqlContext`] (`ctx.data::<GraphqlContext>()`) with:

- `state::<T>()` / `try_state::<T>()` — same types as `app.state(...)`
- `authorization()` — optional `Authorization` header
- `method()`, `path()`

Server-only HTTP handlers use [`GraphqlServerExt::try_graphql_schema()`] to reach the mounted schema.

## Subscriptions

Default WebSocket path: `{api_path}/ws` (override with `.subscriptions("/graphql/ws")`).

Uses `graphql-transport-ws` / `graphql-ws` protocols via `async-graphql` (not `sova-ws` hub).

Auth note: browser WebSocket clients often pass tokens in `connection_init` payload or query string — validate in subscription resolvers via context.

## Modes

| Mode | Install | Handler API |
|------|---------|-------------|
| Client only | `GraphQl::client(url)` / `GraphQl::fake(...)` | `req.graphql()` |
| Server only | `GraphQl::server(schema)` | mount — `try_graphql()` for outbound |
| Composite (BFF) | `GraphQl::server(schema).with_client(url)` | mount + `req.graphql()` |

Optional: `.allow_get_queries(true)`, `.sdl_path("/graphql/sdl")`, `.without_subscriptions()`.

Example: [`examples/api/api_graphql_server`](https://github.com/s00d/sova/tree/master/examples/api/api_graphql_server).

## DevTools

With the [`devtools`](/guide/devtools) feature, GraphQL server operations show up automatically:

| Surface | What you see |
|---------|----------------|
| **GraphQL tab** | Operation name, kind (`query` / `mutation`), duration, error count |
| **Config tab** | Mounted paths (`api`, `graphiql`, `subscriptions`, `sdl`) |
| **Events tab** | WebSocket upgrade events (`graphql.ws.upgrade`) |
| **GraphiQL page** | Bottom DevTools bar (HTML inject) |

Install DevTools **before or after** `GraphQl::server` — mount paths register via shared [`DevToolsConfigRegistry`].

Combined demo: `cargo run -p devtools_demo` → `/graphiql` + GraphQL tab on `POST /graphql`.

## GraphQL client (remote / tests)

```toml
sova = { version = "0.1", features = ["graphql", "testing"] }
```

```rust
use sova::{FakeGraphql, GraphQl, GraphQlExt};
use serde_json::json;

let fake = FakeGraphql::new().stub("hello", json!({ "hello": "world" }));
app.install(GraphQl::fake(fake));

let data = req.graphql().query("query { hello }").data().await?;
```

Live: `GraphQl::client("https://api.example.com/graphql")` or `[graphql] url=…` / `GRAPHQL_URL`.

## Examples

- [`examples/api/api_graphql`](https://github.com/s00d/sova/tree/master/examples/api/api_graphql)
- [`examples/api/api_graphql_server`](https://github.com/s00d/sova/tree/master/examples/api/api_graphql_server)

## Related

[`http`](/plugins/http) · [`grpc`](/plugins/grpc)
