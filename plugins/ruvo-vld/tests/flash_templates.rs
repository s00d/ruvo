//! Flash template providers (`flash-templates`).

#![cfg(feature = "flash-templates")]

use ruvo_core::{App, Request};
use ruvo_session::{memory_sessions, SessionExt, FLASH_ERRORS, FLASH_OLD, FLASH_STATUS};
use ruvo_templates::{RenderExt, Templates};
use ruvo_vld::{with_flash, with_validation_flash};

#[tokio::test]
async fn with_flash_exposes_errors_old_status() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("page.html"),
        r#"{{ status }}|{{ errors.email }}|{{ old.email }}"#,
    )
    .unwrap();

    let mut app = App::new();
    app.install(memory_sessions());
    app.install(with_flash(
        Templates::minijinja(dir.path()).autoreload(false),
    ));
    app.get("/seed", |req: Request| async move {
        req.session().flash(FLASH_STATUS, "Saved");
        req.session()
            .flash(FLASH_ERRORS, r#"{"email":"bad"}"#);
        req.session()
            .flash(FLASH_OLD, r#"{"email":"a@b.c"}"#);
        ruvo_core::Response::text("ok")
    });
    app.get("/page", |req: Request| async move {
        req.render("page.html", serde_json::json!({}))
    });

    let _ = with_validation_flash(Templates::minijinja(dir.path()).autoreload(false));

    let c = ruvo_core::TestClient::tracked(app).unwrap();
    c.get("/seed").await;
    let res = c.get("/page").await;
    assert_eq!(res.status_code().as_u16(), 200);
    let body = String::from_utf8(res.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("Saved"), "{body}");
    assert!(body.contains("bad"), "{body}");
    assert!(body.contains("a@b.c"), "{body}");

    let again = c.get("/page").await;
    let body2 = String::from_utf8(again.body_bytes().unwrap().to_vec()).unwrap();
    assert!(!body2.contains("Saved"), "{body2}");
}
