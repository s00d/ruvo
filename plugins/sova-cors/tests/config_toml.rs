//! `[cors]` unset-fill: origin / origins → ACAO.

use http::Method;
use sova_core::{App, Request, Response};
use sova_cors::Cors;

#[tokio::test]
async fn cors_origin_from_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[cors]
origin = "https://app.toml.test"
"#,
    )
    .unwrap();
    app.install(Cors::new());
    app.get("/", |_r: Request| async { Response::text("ok") });

    let ok = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("origin", "https://app.toml.test")
                .build(),
        )
        .await;
    assert_eq!(
        ok.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://app.toml.test")
    );
}

#[tokio::test]
async fn cors_origins_list_from_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[cors]
origins = ["https://a.toml", "https://b.toml"]
"#,
    )
    .unwrap();
    app.install(Cors::new());
    app.get("/", |_r: Request| async { Response::text("ok") });

    let res = app
        .handle(
            Request::builder()
                .method(Method::GET)
                .path("/")
                .header("origin", "https://b.toml")
                .build(),
        )
        .await;
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://b.toml")
    );
}
