//! gRPC error envelope + composite client/server tests.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sova_core::{App, Request, ResponseAssert, TestClient};
use sova_grpc::{FakeGrpc, Grpc, GrpcError, GrpcExt};

#[derive(Serialize, Deserialize)]
struct HelloIn {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct HelloOut {
    message: String,
}

#[tokio::test]
async fn server_returns_connect_error_envelope() {
    let mut app = App::new();
    app.install(
        Grpc::server().unary("hello.Greeter/SayHello", |req: HelloIn| async move {
            Ok(HelloOut { message: req.name })
        }),
    );

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/hello.Greeter/SayHello")
        .header("content-type", "application/json")
        .body("{")
        .await;
    res.assert_status(400);
    let body = res.json_value();
    assert_eq!(body["code"], "invalid_argument");
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn client_parses_connect_error() {
    let fake = FakeGrpc::new();
    let mut app = App::new();
    app.install(Grpc::fake(fake));
    app.get("/err", |req: Request| async move {
        let err = req
            .grpc()
            .call::<HelloIn, HelloOut>("missing.Method", &HelloIn { name: "x".into() })
            .await
            .unwrap_err();
        match err {
            GrpcError::NotFound(_) => sova_core::Json(json!({ "ok": true })),
            other => panic!("expected NotFound, got {other}"),
        }
    });

    let c = TestClient::new(app).unwrap();
    c.get("/err").await.assert_status(200);
}

#[tokio::test]
async fn composite_server_and_client() {
    let mut app = App::new();
    app.install(
        Grpc::server()
            .unary("hello.Greeter/SayHello", |req: HelloIn| async move {
                Ok(HelloOut {
                    message: format!("hi {}", req.name),
                })
            })
            .client("http://127.0.0.1:1"),
    );
    app.get("/has-client", |req: Request| async move {
        let _ = req.try_grpc().expect("outbound client installed");
        sova_core::Json(json!({ "ok": true }))
    });

    let c = TestClient::new(app).unwrap();
    c.get("/has-client").await.assert_status(200);
}

#[tokio::test]
async fn unary_with_request_sees_header() {
    let mut app = App::new();
    app.install(Grpc::server().unary_with_request(
        "hello.Greeter/SayHello",
        |http: Request, body: HelloIn| async move {
            let who = http.header("x-user").unwrap_or("anon");
            Ok(HelloOut {
                message: format!("hi {} ({})", body.name, who),
            })
        },
    ));

    let c = TestClient::new(app).unwrap();
    let res = c
        .post("/hello.Greeter/SayHello")
        .header("x-user", "admin")
        .json(&HelloIn { name: "bob".into() })
        .await;
    res.assert_status(200);
    assert_eq!(res.json_value()["message"], "hi bob (admin)");
}
