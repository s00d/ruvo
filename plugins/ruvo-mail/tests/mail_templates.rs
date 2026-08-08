//! Mail + MiniJinja templates (feature `templates`).

#![cfg(feature = "templates")]

use ruvo_core::{App, Plugin, Request, Response};
use ruvo_mail::{Mail, MailExt};
use ruvo_templates::Templates;
use serde_json::json;
use tempfile::tempdir;

#[tokio::test]
async fn view_with_layout_and_params() {
    let dir = tempdir().unwrap();
    let views = dir.path();
    std::fs::create_dir(views.join("mail")).unwrap();
    std::fs::write(
        views.join("mail/layout.html"),
        r#"<!DOCTYPE html><html><body>{% block content %}{% endblock %}</body></html>"#,
    )
    .unwrap();
    std::fs::write(
        views.join("mail/welcome.html"),
        r#"{% extends "mail/layout.html" %}
{% block content %}<p>Hello {{ name }}!</p><a href="{{ link }}">Confirm</a>{% endblock %}"#,
    )
    .unwrap();

    let plugin = Mail::fake().from("App <noreply@test.local>");
    let client = plugin.client();
    let mut app = App::new();
    app.install(Templates::minijinja(views).autoreload(false));
    plugin.install(&mut app);

    app.post("/send", |req: Request| async move {
        req.mail()
            .to("user@example.com")
            .subject("Welcome")
            .view(
                "mail/welcome.html",
                json!({ "name": "Ada", "link": "https://example.com/v" }),
            )
            .send()
            .await?;
        Ok::<_, ruvo_core::Error>(Response::text("ok"))
    });

    let res = app
        .handle(
            Request::builder()
                .method(http::Method::POST)
                .path("/send")
                .build(),
        )
        .await;
    assert_eq!(res.status_code().as_u16(), 200, "{:?}", res.body_bytes());

    let sent = client.fake().unwrap().sent();
    assert_eq!(sent.len(), 1);
    let html = sent[0].html.as_deref().unwrap();
    assert!(html.contains("Hello Ada!"), "{html}");
    assert!(
        html.contains("example.com") && html.contains("Confirm"),
        "{html}"
    );
    assert!(html.contains("<!DOCTYPE html>"), "{html}");
}

#[tokio::test]
async fn view_from_compose_without_request() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("ping.html"), r#"<b>{{ msg }}</b>"#).unwrap();

    let mut app = App::new();
    app.install(Templates::minijinja(dir.path()).autoreload(false));
    let plugin = Mail::fake().from("a@b.c");
    let client = plugin.client();
    plugin.install(&mut app);

    client
        .compose()
        .to("u@example.com")
        .subject("Ping")
        .view("ping.html", json!({ "msg": "hi" }))
        .send()
        .await
        .unwrap();

    let html = client.fake().unwrap().sent()[0].html.clone().unwrap();
    assert_eq!(html, "<b>hi</b>");
}
