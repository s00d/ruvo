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
