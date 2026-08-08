//! Shield security headers (integration).

use http::Method;
use sova_core::{App, Request, Response};
use sova_shield::Shield;

#[tokio::test]
async fn default_headers() {
    let mut app = App::new();
    app.install(Shield::default());
    app.get("/", |_r: Request| async { Response::text("ok") });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(
        res.headers()
            .get("x-frame-options")
            .and_then(|v| v.to_str().ok()),
        Some("SAMEORIGIN")
    );
    assert_eq!(
        res.headers()
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        res.headers()
            .get("referrer-policy")
            .and_then(|v| v.to_str().ok()),
        Some("no-referrer")
    );
    assert_eq!(
        res.headers()
            .get("cross-origin-opener-policy")
            .and_then(|v| v.to_str().ok()),
        Some("same-origin")
    );
    assert_eq!(
        res.headers()
            .get("cross-origin-resource-policy")
            .and_then(|v| v.to_str().ok()),
        Some("same-origin")
    );
    assert_eq!(
        res.headers()
            .get("x-dns-prefetch-control")
            .and_then(|v| v.to_str().ok()),
        Some("off")
    );
    assert_eq!(
        res.headers()
            .get("x-download-options")
            .and_then(|v| v.to_str().ok()),
        Some("noopen")
    );
    assert_eq!(
        res.headers()
            .get("x-permitted-cross-domain-policies")
            .and_then(|v| v.to_str().ok()),
        Some("none")
    );
    assert_eq!(
        res.headers()
            .get("x-xss-protection")
            .and_then(|v| v.to_str().ok()),
        Some("0")
    );
    assert!(res.headers().get("content-security-policy").is_none());
}

#[tokio::test]
async fn csp_header() {
    let mut app = App::new();
    app.install(Shield::new().csp("default-src 'self'"));
    app.get("/", |_r: Request| async { Response::text("ok") });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(
        res.headers()
            .get("content-security-policy")
            .and_then(|v| v.to_str().ok()),
        Some("default-src 'self'")
    );
}

#[tokio::test]
async fn frame_off_omits_x_frame_options() {
    let mut app = App::new();
    app.install(Shield::new().frame_off());
    app.get("/", |_r: Request| async { Response::text("ok") });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert!(res.headers().get("x-frame-options").is_none());
}

#[tokio::test]
async fn toml_shield_csp_fills_when_unset() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[shield]
csp = "default-src 'none'"
"#,
    )
    .unwrap();
    app.install(Shield::new());
    app.get("/", |_r: Request| async { Response::text("ok") });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(
        res.headers()
            .get("content-security-policy")
            .and_then(|v| v.to_str().ok()),
        Some("default-src 'none'")
    );
}

#[tokio::test]
async fn explicit_csp_wins_over_toml() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[shield]
csp = "default-src 'none'"
"#,
    )
    .unwrap();
    app.install(Shield::new().csp("default-src 'self'"));
    app.get("/", |_r: Request| async { Response::text("ok") });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(
        res.headers()
            .get("content-security-policy")
            .and_then(|v| v.to_str().ok()),
        Some("default-src 'self'")
    );
}

#[tokio::test]
async fn builder_setters_and_offs() {
    let mut app = App::new();
    app.install(
        Shield::new()
            .frame("DENY")
            .content_type("nosniff")
            .referrer("origin")
            .cross_origin_opener("same-origin-allow-popups")
            .cross_origin_resource("cross-origin")
            .dns_prefetch("on")
            .download_options("noopen")
            .permitted_cross_domain("none")
            .xss_protection("0")
            .csp("default-src 'self'"),
    );
    app.get("/", |_r: Request| async { Response::text("ok") });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(
        res.headers()
            .get("x-frame-options")
            .and_then(|v| v.to_str().ok()),
        Some("DENY")
    );
    assert_eq!(
        res.headers()
            .get("referrer-policy")
            .and_then(|v| v.to_str().ok()),
        Some("origin")
    );
    assert_eq!(
        res.headers()
            .get("cross-origin-opener-policy")
            .and_then(|v| v.to_str().ok()),
        Some("same-origin-allow-popups")
    );
    assert_eq!(
        res.headers()
            .get("content-security-policy")
            .and_then(|v| v.to_str().ok()),
        Some("default-src 'self'")
    );

    let mut app2 = App::new();
    app2.install(
        Shield::new()
            .frame_off()
            .content_type_off()
            .referrer_off()
            .cross_origin_opener_off()
            .cross_origin_resource_off()
            .dns_prefetch_off()
            .download_options_off()
            .permitted_cross_domain_off()
            .xss_protection_off()
            .csp_off(),
    );
    app2.get("/", |_r: Request| async { Response::text("ok") });
    let res2 = app2.handle_request(Method::GET, "/", "").await;
    assert!(res2.headers().get("x-frame-options").is_none());
    assert!(res2.headers().get("x-content-type-options").is_none());
    assert!(res2.headers().get("referrer-policy").is_none());
    assert!(res2.headers().get("cross-origin-opener-policy").is_none());
    assert!(res2.headers().get("cross-origin-resource-policy").is_none());
    assert!(res2.headers().get("x-dns-prefetch-control").is_none());
    assert!(res2.headers().get("x-download-options").is_none());
    assert!(res2
        .headers()
        .get("x-permitted-cross-domain-policies")
        .is_none());
    assert!(res2.headers().get("x-xss-protection").is_none());
    assert!(res2.headers().get("content-security-policy").is_none());
}

#[tokio::test]
async fn toml_frame_fills() {
    let mut app = App::new();
    app.configure_from_str(
        r#"
[shield]
frame = "DENY"
"#,
    )
    .unwrap();
    app.install(Shield::new());
    app.get("/", |_r: Request| async { Response::text("ok") });
    let res = app.handle_request(Method::GET, "/", "").await;
    assert_eq!(
        res.headers()
            .get("x-frame-options")
            .and_then(|v| v.to_str().ok()),
        Some("DENY")
    );
}
