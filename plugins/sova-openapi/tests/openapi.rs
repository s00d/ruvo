use http::Method;
use serde_json::json;
use sova_core::{App, Request, Response};
use sova_openapi::{build_document, undocumented, BuildOptions, Doc, OpenApi, OpenApiDocExt};

#[tokio::test]
async fn doc_stored_on_route_meta() {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") })
        .doc(Doc::new().ok_schema(json!({ "type": "string" })));
    let entries = app.route_entries();
    let meta = match &entries[0] {
        sova_core::extend::RouteEntry::Http { meta, .. } => meta,
        _ => panic!("expected http"),
    };
    assert!(meta.get::<Doc>().is_some());
}

#[tokio::test]
async fn openapi_json_from_route_table() {
    let mut app = App::new();
    app.get("/users/:id", |_r: Request| async { Response::text("u") })
        .doc(
            Doc::new()
                .ok_schema(json!({ "type": "string" }))
                .body_schema(json!({ "type": "object" })),
        );
    app.install(OpenApi::new("Test", "1.0").mount("/docs"));

    let server = app.build().unwrap();
    let res = server
        .handle_request(Method::GET, "/docs/openapi.json", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    let bytes = res.body_bytes().unwrap();
    let v: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(v["openapi"], "3.1.0");
    assert!(v["paths"].get("/users/{id}").is_some());
    let get = &v["paths"]["/users/{id}"]["get"];
    assert!(get["responses"]["422"].is_object());
    assert_eq!(get["parameters"][0]["name"], "id");
}

#[tokio::test]
async fn undocumented_lists_missing_docs() {
    let mut app = App::new();
    app.get("/a", |_r: Request| async { Response::text("a") });
    app.get("/b", |_r: Request| async { Response::text("b") })
        .doc(Doc::skip());
    app.install(OpenApi::new("T", "1").mount("/docs"));
    let missing = undocumented(&app);
    assert!(missing.iter().any(|s| s.contains("/a")));
    assert!(!missing.iter().any(|s| s.contains("/b")));
    assert!(!missing.iter().any(|s| s.contains("/docs")));
}

#[tokio::test]
async fn mount_and_server_from_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[openapi]
mount = "/api-docs"
server = "https://api.example.com"
"#,
    )
    .unwrap();
    app.get("/ping", |_r: Request| async { Response::text("ok") })
        .doc(Doc::new().ok_schema(json!({ "type": "string" })));
    app.install(OpenApi::new("Toml API", "0.1"));

    let server = app.build().unwrap();
    let res = server
        .handle_request(Method::GET, "/api-docs/openapi.json", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    let v: serde_json::Value = serde_json::from_slice(res.body_bytes().unwrap()).unwrap();
    assert_eq!(v["info"]["title"], "Toml API");
    assert_eq!(v["servers"][0]["url"], "https://api.example.com");
    assert!(v["paths"].get("/ping").is_some());
}

#[test]
fn brace_wildcards_skipped_in_document() {
    let mut meta = sova_core::extend::MetaMap::new();
    meta.insert(Doc::new().ok_schema(json!({})));
    let table = sova_core::extend::RouteTable(vec![sova_core::extend::RouteEntry::Http {
        method: Method::GET,
        path: "/files/*path".into(),
        meta,
    }]);
    let doc = build_document(
        &table,
        &BuildOptions {
            title: "t",
            version: "1",
            servers: &[],
            docs_prefix: "/docs",
        },
    );
    assert!(doc["paths"].as_object().unwrap().is_empty());
}

#[tokio::test]
async fn local_assets_and_trailing_slash_mount() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("scalar.js"), b"/* scalar */").unwrap();

    let mut app = App::new();
    app.get("/ping", |_r: Request| async { Response::text("ok") })
        .doc(Doc::new().ok_schema(json!({ "type": "string" })));
    app.install(
        OpenApi::new("Local", "1")
            .mount("/docs/")
            .server("http://localhost")
            .local_assets(dir.path()),
    );

    let server = app.build().unwrap();
    let html = server.handle_request(Method::GET, "/docs", "").await;
    assert_eq!(html.status_code().as_u16(), 200);
    let body = String::from_utf8(html.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("/docs/scalar.js"), "{body}");
    assert!(body.contains("data-url=\"/docs/openapi.json\""));

    let json = server
        .handle_request(Method::GET, "/docs/openapi.json", "")
        .await;
    assert_eq!(json.status_code().as_u16(), 200);
}

#[tokio::test]
async fn openapi_audit_fails_on_undocumented() {
    use sova_core::CheckKind;

    let mut app = App::new();
    app.get("/a", |_r: Request| async { Response::text("a") });
    app.install(OpenApi::new("T", "1").mount("/docs"));
    let server = app.build().unwrap();
    let results = app.run_checks(server.state(), &[CheckKind::Audit]).await;
    assert!(
        results.iter().any(|r| r.name == "openapi" && !r.ok),
        "{results:?}"
    );
}

#[test]
fn build_document_covers_methods_query_and_status() {
    use sova_core::extend::{MetaMap, RouteEntry, RouteTable};

    fn entry(method: Method, path: &str, doc: Doc) -> RouteEntry {
        let mut meta = MetaMap::new();
        meta.insert(doc);
        RouteEntry::Http {
            method,
            path: path.into(),
            meta,
        }
    }

    let table = RouteTable(vec![
        entry(
            Method::POST,
            "/items",
            Doc::new()
                .body_schema(json!({ "type": "object" }))
                .query_schema(json!({
                    "type": "object",
                    "required": ["q"],
                    "properties": { "q": { "type": "string" } }
                }))
                .response(201, json!({ "type": "object" }))
                .response(204, json!({}))
                .response(400, json!({}))
                .response(401, json!({}))
                .response(404, json!({}))
                .response(422, json!({}))
                .response(418, json!({}))
                .ok_schema(json!({ "type": "string" })),
        ),
        entry(Method::PUT, "/items/:id", Doc::new().ok_schema(json!({}))),
        entry(Method::PATCH, "/items/:id", Doc::new().ok_schema(json!({}))),
        entry(
            Method::DELETE,
            "/items/:id",
            Doc::new().ok_schema(json!({})),
        ),
        entry(Method::HEAD, "/items", Doc::new().ok_schema(json!({}))),
        entry(Method::OPTIONS, "/items", Doc::new().ok_schema(json!({}))),
        // Non-object query schema fallback.
        entry(
            Method::GET,
            "/raw",
            Doc::new().query_schema(json!({ "type": "string" })),
        ),
    ]);

    let doc = build_document(
        &table,
        &BuildOptions {
            title: "t",
            version: "1",
            servers: &["https://api.example".into()],
            docs_prefix: "/docs",
        },
    );
    assert_eq!(doc["servers"][0]["url"], "https://api.example");
    assert!(doc["paths"]["/items"]["post"]["requestBody"].is_object());
    assert!(doc["paths"]["/items"]["post"]["responses"]["201"].is_object());
    assert!(doc["paths"]["/items/{id}"]["put"].is_object());
    assert!(doc["paths"]["/items/{id}"]["patch"].is_object());
    assert!(doc["paths"]["/items/{id}"]["delete"].is_object());
    assert!(doc["paths"]["/items"]["head"].is_object());
    assert!(doc["paths"]["/items"]["options"].is_object());
    assert_eq!(
        doc["paths"]["/raw"]["get"]["parameters"][0]["name"],
        "query"
    );
}
