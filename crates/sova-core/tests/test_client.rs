use sova_core::{App, Request, Response, ResponseAssert, TestClient};

#[tokio::test]
async fn cookie_jar_round_trip() {
    let mut app = App::new();
    app.get("/set", |_r: Request| async {
        Response::text("ok").header("set-cookie", "sid=abc123; Path=/; HttpOnly")
    });
    app.get("/echo", |req: Request| async move {
        let c = req.header("cookie").unwrap_or("").to_string();
        Response::text(c)
    });

    let c = TestClient::tracked(app).await.unwrap();
    let _ = c.get("/set").await;
    let res = c.get("/echo").await;
    assert_eq!(res.body_bytes(), Some(b"sid=abc123".as_slice()));
}

#[tokio::test]
async fn on_request_hook_injects_extension() {
    #[derive(Clone)]
    struct Marker(u64);

    let mut app = App::new();
    app.get("/who", |req: Request| async move {
        let n = req.get::<Marker>().map(|m| m.0).unwrap_or(0);
        Response::json(&serde_json::json!({ "n": n }))
    });

    let c = TestClient::tracked(app).await.unwrap();
    c.on_request(|req| {
        req.set(Marker(42));
    });
    let res = c.get("/who").await;
    res.assert_status(200);
    assert_eq!(res.json_value()["n"], 42);

    c.clear_request_hooks();
    let res2 = c.get("/who").await;
    assert_eq!(res2.json_value()["n"], 0);
}
