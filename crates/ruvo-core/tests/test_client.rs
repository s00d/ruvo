use ruvo_core::{App, Request, Response, TestClient};

#[tokio::test]
async fn cookie_jar_round_trip() {
    let mut app = App::new();
    app.get("/set", |_r: Request| async {
        Response::text("ok").header(
            "set-cookie",
            "sid=abc123; Path=/; HttpOnly",
        )
    });
    app.get("/echo", |req: Request| async move {
        let c = req.header("cookie").unwrap_or("").to_string();
        Response::text(c)
    });

    let c = TestClient::tracked(app).unwrap();
    let _ = c.get("/set").await;
    let res = c.get("/echo").await;
    assert_eq!(res.body_bytes(), Some(b"sid=abc123".as_slice()));
}
