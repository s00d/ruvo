//! gRPC-style Connect-JSON client demo (fake by default).

use serde::{Deserialize, Serialize};
use serde_json::json;
use sova::{App, FakeGrpc, Grpc, GrpcExt, Json, Request, Result};

#[derive(Serialize)]
struct HelloIn {
    name: String,
}

#[derive(Deserialize)]
struct HelloOut {
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    if let Ok(url) = std::env::var("GRPC_URL") {
        app.install(Grpc::client(url));
    } else {
        let fake = FakeGrpc::new().stub_json(
            "hello.Greeter/SayHello",
            json!({ "message": "hi from sova-grpc (fake)" }),
        );
        app.install(Grpc::fake(fake));
    }

    app.get("/api/hello", |req: Request| async move {
        let out: HelloOut = req
            .grpc()
            .call(
                "hello.Greeter/SayHello",
                &HelloIn {
                    name: "sova".into(),
                },
            )
            .await?;
        Ok::<_, sova::Error>(Json(json!({ "message": out.message })))
    });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    eprintln!("api_grpc listening on http://127.0.0.1:{port}");
    app.listen(port).await
}
