use http::Method;
use sova_core::{App, IntoResponse, Request, Response};
use sova_vld::ValidationExt;

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

sova_vld::doc_schema!(CreateUser, IdParams);

#[tokio::test]
async fn validate_body_hook_and_valid() {
    use sova_vld::{ValidExt, ValidateRouteExt};

    let mut app = App::new();
    app.post("/users", |req: Request| async move {
        let u = req.valid::<CreateUser>();
        Response::text(u.name.clone())
    })
    .validate_body::<CreateUser>();

    let server = app.build().unwrap();
    let ok = server
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/users")
                .body(r#"{"name":"Alex","email":"a@b.co"}"#)
                .build(),
        )
        .await;
    assert_eq!(ok.body_bytes(), Some(b"Alex".as_slice()));

    let bad = server
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/users")
                .body(r#"{"name":"A","email":"bad"}"#)
                .build(),
        )
        .await;
    assert_eq!(bad.status_code().as_u16(), 422);
}

#[tokio::test]
async fn validate_params_coerces_via_schema() {
    use sova_vld::{ValidExt, ValidateRouteExt};

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct NumId {
            pub id: f64 => vld::number().min(1.0),
        }
    }
    sova_vld::doc_schema!(NumId);

    let mut app = App::new();
    app.get("/n/:id", |req: Request| async move {
        let p = req.valid::<NumId>();
        Response::text((p.id as i64).to_string())
    })
    .validate_params::<NumId>();

    let server = app.build().unwrap();
    let res = server.handle_request(Method::GET, "/n/42", "").await;
    assert_eq!(res.body_bytes(), Some(b"42".as_slice()));
}

#[tokio::test]
async fn openapi_reads_validate_meta() {
    use sova_openapi::{build_document, BuildOptions, OpenApiValidate};
    use sova_vld::ValidateRouteExt;

    let mut app = App::new();
    app.post("/users", |_r: Request| async { Response::text("ok") })
        .validate_body::<CreateUser>();

    let entries = app.route_entries();
    let meta = match &entries[0] {
        sova_core::extend::RouteEntry::Http { meta, .. } => meta,
        _ => panic!("http"),
    };
    assert!(meta.get::<OpenApiValidate>().unwrap().body.is_some());

    let table = sova_core::extend::RouteTable(entries);
    let doc = build_document(
        &table,
        &BuildOptions {
            title: "t",
            version: "1",
            servers: &[],
            docs_prefix: "/docs",
        },
    );
    assert!(doc["paths"]["/users"]["post"]["requestBody"].is_object());
}

#[tokio::test]
async fn nested_query_via_serde_qs() {
    use sova_vld::ValidationExt;

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct Filter {
            pub name: String => vld::string().min(1),
        }
    }
    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct NestedQ {
            pub filter: Filter => vld::nested(Filter::parse_value),
        }
    }

    let req = Request::builder()
        .method(Method::GET)
        .path("/?filter[name]=ada")
        .build();
    let q: NestedQ = req.validate_query().unwrap();
    assert_eq!(q.filter.name, "ada");
}

#[tokio::test]
async fn validate_all_params_override_body() {
    use sova_vld::{ValidExt, ValidateRouteExt};

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct PatchUser {
            pub id: String => vld::string().min(1),
            pub name: String => vld::string().min(1),
        }
    }
    sova_vld::doc_schema!(PatchUser);

    let mut app = App::new();
    app.post("/u/:id", |req: Request| async move {
        let u = req.valid::<PatchUser>();
        Response::text(format!("{}:{}", u.id, u.name))
    })
    .validate_all::<PatchUser>();

    let server = app.build().unwrap();
    let res = server
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/u/from-path")
                .body(r#"{"id":"from-body","name":"Ada"}"#)
                .build(),
        )
        .await;
    assert_eq!(res.body_bytes(), Some(b"from-path:Ada".as_slice()));
}

#[tokio::test]
async fn missing_validate_routes_coverage() {
    use sova_core::extend::RouteTable;
    use sova_vld::{missing_validate_routes, ValidateRouteExt};

    let mut bare = App::new();
    bare.post("/x", |_r: Request| async { Response::text("ok") });
    let bare_missing = missing_validate_routes(&RouteTable(bare.route_entries()));
    assert!(
        bare_missing.iter().any(|s| s.contains("POST") && s.contains("/x")),
        "{bare_missing:?}"
    );

    let mut ok = App::new();
    ok.post("/x", |_r: Request| async { Response::text("ok") })
        .validate_body::<CreateUser>();
    let ok_missing = missing_validate_routes(&RouteTable(ok.route_entries()));
    assert!(ok_missing.is_empty(), "{ok_missing:?}");
}

#[cfg(feature = "flash")]
#[tokio::test]
async fn flash_html_accept_redirects() {
    use sova_vld::ValidateRouteExt;

    let mut app = App::new();
    app.post("/users", |_r: Request| async { Response::text("ok") })
        .validate_body::<CreateUser>();

    let server = app.build().unwrap();

    let html = server
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/users")
                .header("accept", "text/html")
                .header("referer", "/form")
                .body(r#"{"name":"A","email":"bad"}"#)
                .build(),
        )
        .await;
    assert_eq!(html.status_code().as_u16(), 303);
    assert_eq!(
        html.headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/form")
    );

    let json = server
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/users")
                .header("accept", "application/json")
                .body(r#"{"name":"A","email":"bad"}"#)
                .build(),
        )
        .await;
    assert_eq!(json.status_code().as_u16(), 422);
}

#[cfg(feature = "form")]
#[tokio::test]
async fn validate_form_multipart_text() {
    use bytes::Bytes;
    use sova_vld::{ValidExt, ValidateRouteExt};

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct FormUser {
            pub name: String => vld::string().min(2),
            pub email: String => vld::string().email(),
        }
    }
    sova_vld::doc_schema!(FormUser);

    let mut app = App::new();
    app.post("/users", |req: Request| async move {
        let u = req.valid::<FormUser>();
        Response::text(u.name.clone())
    })
    .validate_form::<FormUser>();

    let boundary = "----sovaBound";
    let parts = concat!(
        "Content-Disposition: form-data; name=\"name\"\r\n\r\n",
        "Alex\r\n",
        "------sovaBound\r\n",
        "Content-Disposition: form-data; name=\"email\"\r\n\r\n",
        "a@b.co\r\n",
    );
    let body = Bytes::from(format!("--{boundary}\r\n{parts}--{boundary}--\r\n"));

    let server = app.build().unwrap();
    let res = server
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/users")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(body)
                .build(),
        )
        .await;
    assert_eq!(res.body_bytes(), Some(b"Alex".as_slice()));
}

#[cfg(feature = "form")]
#[tokio::test]
async fn validation_ext_validate_form_urlencoded() {
    use sova_vld::ValidationExt;

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct NoteForm {
            pub title: String => vld::string().min(1),
            pub body: String => vld::string(),
        }
    }

    let mut app = App::new();
    app.post("/notes", |mut req: Request| async move {
        let form: NoteForm = req.validate_form().await.unwrap();
        Response::text(form.title)
    });

    let server = app.build().unwrap();
    let res = server
        .handle(
            Request::builder()
                .method(Method::POST)
                .path("/notes")
                .header("content-type", "application/x-www-form-urlencoded")
                .body("title=Hello&body=World")
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    assert_eq!(res.body_bytes(), Some(b"Hello".as_slice()));
}

#[tokio::test]
async fn vld_plugin_audit_and_validate_query_route() {
    use sova_core::CheckKind;
    use sova_vld::{ValidExt, ValidateRouteExt, Vld};

    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct Search {
            pub q: String => vld::string().min(1),
        }
    }
    sova_vld::doc_schema!(Search);

    let mut bare = App::new();
    bare.post("/x", |_r: Request| async { Response::text("ok") });
    bare.install(Vld);
    let server = bare.build().unwrap();
    let results = bare
        .run_checks(server.state(), &[CheckKind::Audit])
        .await;
    assert!(results.iter().any(|r| r.name == "vld" && !r.ok));

    let mut app = App::new();
    app.get("/search", |req: Request| async move {
        let q = req.valid::<Search>();
        Response::text(q.q.clone())
    })
    .validate_query::<Search>();
    app.install(Vld);

    let server = app.build().unwrap();
    let ok = server
        .handle_request(Method::GET, "/search?q=hi", "")
        .await;
    assert_eq!(ok.body_bytes(), Some(b"hi".as_slice()));

    // Coercion path (openapi feature): numeric string → number via schema.
    vld::schema! {
        #[derive(Debug, Clone)]
        pub struct NumQ {
            pub n: f64 => vld::number().min(1.0),
        }
    }
    sova_vld::doc_schema!(NumQ);
    let mut app2 = App::new();
    app2.get("/n", |req: Request| async move {
        let q = req.valid::<NumQ>();
        Response::text((q.n as i64).to_string())
    })
    .validate_query::<NumQ>();
    let s2 = app2.build().unwrap();
    let res = s2.handle_request(Method::GET, "/n?n=3", "").await;
    assert_eq!(res.body_bytes(), Some(b"3".as_slice()));
}
