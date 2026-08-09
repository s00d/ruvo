//! Extractor handlers + legacy `Fn(Request)` regression.

use http::Method;
use serde::{Deserialize, Serialize};
use sova_core::extract::{Json, Path, State};
use sova_core::{App, Cell, Request, Response};

#[derive(Clone)]
struct Counter {
    n: Cell<u64>,
}

impl Default for Counter {
    fn default() -> Self {
        Self { n: Cell::new(0) }
    }
}

#[derive(Deserialize)]
struct IdPath {
    id: String,
}

#[derive(Deserialize, Serialize)]
struct Body {
    msg: String,
}

#[tokio::test]
async fn extractors_path_json_state() {
    let mut app = App::new();
    app.state(Counter::default());
    app.post("/echo/:id", echo);

    let res = app
        .handle_request(Method::POST, "/echo/42", r#"{"msg":"hi"}"#)
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    let v: serde_json::Value = serde_json::from_slice(res.body_bytes().unwrap()).unwrap();
    assert_eq!(v["id"], "42");
    assert_eq!(v["msg"], "hi");
    assert_eq!(v["n"], 1);
}

async fn echo(
    Path(IdPath { id }): Path<IdPath>,
    Json(Body { msg }): Json<Body>,
    State(counter): State<Counter>,
) -> Response {
    counter.n.update(|v| v + 1);
    let n = counter.n.get();
    Response::json(&serde_json::json!({ "id": id, "msg": msg, "n": n }))
}

#[tokio::test]
async fn legacy_request_handler_still_works() {
    let mut app = App::new();
    app.get("/ping", |req: Request| async move {
        format!("pong:{}", req.path)
    });
    let res = app.handle_request(Method::GET, "/ping", "").await;
    assert_eq!(res.status_code().as_u16(), 200);
    let body = String::from_utf8_lossy(res.body_bytes().unwrap());
    assert_eq!(body, "pong:/ping");
}
