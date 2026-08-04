use http::Method;
use ruvo_core::{App, IntoResponse, Request, Response};
use ruvo_vld::ValidationExt;

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct CreateUser {
        pub name: String => vld::string().min(2).max(50),
        pub email: String => vld::string().email(),
    }
}

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct IdParams {
        pub id: String => vld::string().min(1),
    }
}

#[tokio::test]
async fn validate_ok() {
    let mut req = Request::builder()
        .method(Method::POST)
        .path("/users")
        .body(r#"{"name":"Alex","email":"a@b.co"}"#)
        .build();
    let user: CreateUser = req.validate().await.unwrap();
    assert_eq!(user.name, "Alex");
}

#[tokio::test]
async fn validate_bad_json_is_400() {
    let mut req = Request::builder()
        .method(Method::POST)
        .path("/")
        .body("{not-json")
        .build();
    let err = req.validate::<CreateUser>().await.unwrap_err();
    assert_eq!(err.status_code(), 400);
    let res = err.into_response();
    assert_eq!(res.status_code().as_u16(), 400);
}

#[tokio::test]
async fn validate_rules_is_422_with_many_issues() {
    let mut req = Request::builder()
        .method(Method::POST)
        .path("/")
        .body(r#"{"name":"A","email":"bad"}"#)
        .build();
    let err = req.validate::<CreateUser>().await.unwrap_err();
    assert_eq!(err.status_code(), 422);
    assert!(err.0.issues.len() >= 2);
    let res = err.into_response();
    assert_eq!(res.status_code().as_u16(), 422);
    let body = String::from_utf8(res.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("validation_failed"));
    assert!(body.contains("issues"));
}

#[tokio::test]
async fn validate_params_and_query() {
    let mut app = App::new();
    app.get("/u/:id", |req: Request| async move {
        let p: IdParams = match req.validate_params() {
            Ok(p) => p,
            Err(e) => return e.into_response(),
        };
        Response::text(p.id)
    });
    let server = app.build().unwrap();
    let res = server
        .handle_request(Method::GET, "/u/abc", "")
        .await;
    assert_eq!(res.body_bytes(), Some(b"abc".as_slice()));
}

#[tokio::test]
async fn validate_query_smoke() {
    vld::schema! {
        pub struct Q {
            pub q: String => vld::string().min(1),
        }
    }
    let req = Request::builder()
        .method(Method::GET)
        .path("/?q=hi")
        .build();
    let q: Q = req.validate_query().unwrap();
    assert_eq!(q.q, "hi");
}
