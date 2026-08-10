//! Client-first gRPC fake tests.

use serde::{Deserialize, Serialize};
use sova_core::{App, Request, ResponseAssert, TestClient};
use sova_grpc::{FakeGrpc, Grpc, GrpcExt};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct HelloIn {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct HelloOut {
    message: String,
}

#[tokio::test]
async fn fake_client_call() {
    let fake = FakeGrpc::new().stub_json(
        "hello.Greeter/SayHello",
        json!({ "message": "hi bob" }),
    );
    let mut app = App::new();
    app.install(Grpc::fake(fake.clone()));
    app.get("/ping", |req: Request| async move {
        let out: HelloOut = req
            .grpc()
            .call(
                "hello.Greeter/SayHello",
                &HelloIn {
                    name: "bob".into(),
                },
            )
            .await
            .unwrap();
        sova_core::Json(json!({ "message": out.message }))
    });

    let c = TestClient::new(app).unwrap();
    let res = c.get("/ping").await;
    res.assert_status(200);
    assert_eq!(res.json_value()["message"], "hi bob");
    fake.assert_called_method("hello.Greeter/SayHello");
}

#[tokio::test]
async fn server_mount_unary_http() {
    let mut app = App::new();
    app.install(
        Grpc::server().unary("hello.Greeter/SayHello", |req: HelloIn| async move {
            Ok::<_, sova_grpc::GrpcError>(HelloOut {
                message: format!("hi {}", req.name),
            })
        }),
    );

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/hello.Greeter/SayHello")
        .json(&HelloIn {
            name: "ann".into(),
        })
        .await;
    res.assert_status(200);
    assert_eq!(res.json_value()["message"], "hi ann");
}
