---
title: grpc
editLink: false
---

# `grpc`

**Connect-JSON unary RPC client (+ optional server)**

| | |
|--|--|
| Crate | [`sova-grpc`](https://docs.rs/sova-grpc/0.1.0) `0.1.0` |
| Plugin id | `grpc` |
| Category | Integrations |

## Install

```bash
cargo add sova --features grpc
```

## Features

| Feature | What you get |
|---------|-------------|
| `grpc` | Connect-JSON unary RPC client (`req.grpc()`, FakeGrpc). |

## Overview

Connect-JSON unary RPC for Sova — client first, optional server.

```rust
 use sova_grpc::{FakeGrpc, Grpc, GrpcExt};
 use serde::{Deserialize, Serialize};

 #[derive(Serialize)]
 struct HelloIn { name: String }
 #[derive(Deserialize)]
 struct HelloOut { message: String }

 let fake = FakeGrpc::new().stub_json("hello.Greeter/SayHello", serde_json::json!({
     "message": "hi"
 }));
 app.install(Grpc::fake(fake));

 let out: HelloOut = req.grpc().call("hello.Greeter/SayHello", &HelloIn { name: "a".into() }).await?;
 ```

## Quick start

**Audience:** app authors calling unary RPC over Connect-JSON (no tonic / `.proto` required).

## Client

```toml
sova = { version = "0.1", features = ["grpc", "testing"] }
```

```rust
use sova::{FakeGrpc, Grpc, GrpcExt};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize)]
struct HelloIn { name: String }
#[derive(Deserialize)]
struct HelloOut { message: String }

let fake = FakeGrpc::new().stub_json(
    "hello.Greeter/SayHello",
    json!({ "message": "hi" }),
);
app.install(Grpc::fake(fake));

let out: HelloOut = req.grpc()
    .call("hello.Greeter/SayHello", &HelloIn { name: "a".into() })
    .await?;
```

Live: `Grpc::client("http://127.0.0.1:50051")` or `[grpc] client_url=…` / `GRPC_URL`.

## Server (optional)

Mount unary handlers on the main HTTP app (paths `/pkg.Service/Method`), optionally `.bind("127.0.0.1:50051")` as a BackgroundService:

```rust
app.install(
    Grpc::server()
        .unary("hello.Greeter/SayHello", |req: HelloIn| async move {
            Ok(HelloOut { message: format!("hi {}", req.name) })
        }),
);
```

## Examples

- [`examples/api/api_grpc`](https://github.com/s00d/sova/tree/master/examples/api/api_grpc)

## Related

[`http`](/plugins/http) · [`graphql`](/plugins/graphql)
