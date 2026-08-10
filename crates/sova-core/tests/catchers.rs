use http::Method;
use sova_core::{App, Html, Json, Request, Response, Router};

#[tokio::test]
async fn scoped_404_html_vs_json() {
    let mut api = Router::new();
    api.get("/ping", |_r: Request| async { Response::text("pong") });
    api.catch(404, |_r: Request| async {
        Json(serde_json::json!({"error": "not_found"}))
    });

    let mut app = App::new();
    app.get("/", |_r: Request| async {
        Html("<h1>home</h1>".to_string())
    });
    app.not_found(|_r: Request| async { Html("<h1>missing</h1>".to_string()) });
    app.mount("/api", api);

    let html = app.handle_request(Method::GET, "/nope", "").await;
    assert_eq!(html.status_code().as_u16(), 404);
    assert_eq!(html.body_bytes(), Some(b"<h1>missing</h1>".as_slice()));

    let json = app.handle_request(Method::GET, "/api/missing", "").await;
    assert_eq!(json.status_code().as_u16(), 404);
    let body = String::from_utf8(json.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("not_found"));

    let pong = app.handle_request(Method::GET, "/api/ping", "").await;
    assert_eq!(pong.body_bytes(), Some(b"pong".as_slice()));
}

#[tokio::test]
async fn longest_prefix_wins() {
    let mut inner = Router::new();
    inner.catch(404, |_r: Request| async {
        Response::text("inner").status(404)
    });

    let mut api = Router::new();
    api.catch(404, |_r: Request| async {
        Response::text("api").status(404)
    });
    api.mount("/v1", inner);

    let mut app = App::new();
    app.catch(404, |_r: Request| async {
        Response::text("root").status(404)
    });
    app.mount("/api", api);

    assert_eq!(
        app.handle_request(Method::GET, "/gone", "")
            .await
            .body_bytes(),
        Some(b"root".as_slice())
    );
    assert_eq!(
        app.handle_request(Method::GET, "/api/gone", "")
            .await
            .body_bytes(),
        Some(b"api".as_slice())
    );
    assert_eq!(
        app.handle_request(Method::GET, "/api/v1/gone", "")
            .await
            .body_bytes(),
        Some(b"inner".as_slice())
    );
}

#[tokio::test]
async fn catch_replaces_handler_error_status() {
    let mut app = App::new();
    app.catch(500, |_r: Request| async {
        Response::text("caught-500").status(500)
    });
    app.get("/boom", |_r: Request| async {
        Response::text("raw").status(500)
    });

    let res = app.handle_request(Method::GET, "/boom", "").await;
    assert_eq!(res.status_code().as_u16(), 500);
    assert_eq!(res.body_bytes(), Some(b"caught-500".as_slice()));
}

#[tokio::test]
async fn catch_405_when_method_missing() {
    let mut app = App::new();
    app.get("/only-get", |_r: Request| async { Response::text("ok") });
    app.catch(405, |_r: Request| async {
        Response::text("nope-method").status(405)
    });

    let res = app.handle_request(Method::POST, "/only-get", "").await;
    assert_eq!(res.status_code().as_u16(), 405);
    assert_eq!(res.body_bytes(), Some(b"nope-method".as_slice()));
}
