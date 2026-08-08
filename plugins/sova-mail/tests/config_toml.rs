//! `[mail]` unset-fill: toml `from` when builder did not call `.from()`.

use sova_core::{App, Request};
use sova_mail::{Mail, MailExt};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_profile(profile: &str, f: impl FnOnce()) {
    let _guard = ENV_LOCK.lock().unwrap();
    let prev_profile = std::env::var("SOVA_PROFILE").ok();
    let prev_env = std::env::var("SOVA_ENV").ok();
    std::env::set_var("SOVA_PROFILE", profile);
    std::env::remove_var("SOVA_ENV");
    f();
    match prev_profile {
        Some(v) => std::env::set_var("SOVA_PROFILE", v),
        None => std::env::remove_var("SOVA_PROFILE"),
    }
    match prev_env {
        Some(v) => std::env::set_var("SOVA_ENV", v),
        None => std::env::remove_var("SOVA_ENV"),
    }
}

#[test]
fn mail_from_toml_fills_when_unset() {
    with_profile("development", || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut app = App::new();
            app.configure_from_str(
                r#"
[mail]
from = "App <noreply@toml.example>"

[development.mail]
from = "Dev <dev@toml.example>"
"#,
            )
            .unwrap();

            let plugin = Mail::fake();
            let recorder = plugin.recorder().unwrap().clone();
            app.install(plugin);

            app.post("/send", |req: Request| async move {
                req.mail()
                    .to("user@example.com")
                    .subject("Hi")
                    .text("body")
                    .send()
                    .await
                    .unwrap();
                sova_core::Response::text("ok")
            });

            let res = app
                .handle(
                    Request::builder()
                        .method(http::Method::POST)
                        .path("/send")
                        .build(),
                )
                .await;
            assert_eq!(res.status_code().as_u16(), 200);

            let sent = recorder.sent();
            assert_eq!(sent.len(), 1);
            assert_eq!(sent[0].from, "Dev <dev@toml.example>");
        });
    });
}
