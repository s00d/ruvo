use http::Method;
use ruvo_core::{App, Request, Response};
use ruvo_openapi::{build_document, undocumented, BuildOptions, Doc, OpenApi, OpenApiDocExt};
use serde_json::json;

#[tokio::test]
async fn doc_stored_on_route_meta() {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") })
        .doc(Doc::new().ok_schema(json!({ "type": "string" })));
    let entries = app.route_entries();
    let meta = match &entries[0] {
        ruvo_core::extend::RouteEntry::Http { meta, .. } => meta,
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

#[test]
fn brace_wildcards_skipped_in_document() {
    let mut meta = ruvo_core::extend::MetaMap::new();
    meta.insert(Doc::new().ok_schema(json!({})));
    let table = ruvo_core::extend::RouteTable(vec![ruvo_core::extend::RouteEntry::Http {
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
