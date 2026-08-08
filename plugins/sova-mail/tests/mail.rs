//! Mail plugin tests.

use sova_core::{App, Plugin, Request};
use sova_mail::{Email, Mail, MailExt};

#[tokio::test]
async fn fake_records_text_and_html() {
    let plugin = Mail::fake().from("App <noreply@test.local>");
    let client = plugin.client();
    let mut app = App::new();
    plugin.install(&mut app);

    app.post("/send", |req: Request| async move {
        req.mail()
            .to("user@example.com")
            .subject("Welcome")
            .text("Hello")
            .html("<p>Hello</p>")
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

    let sent = client.fake().unwrap().sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, vec!["user@example.com"]);
    assert_eq!(sent[0].subject, "Welcome");
    assert_eq!(sent[0].text.as_deref(), Some("Hello"));
    assert_eq!(sent[0].html.as_deref(), Some("<p>Hello</p>"));
}

#[tokio::test]
async fn invalid_to_is_bad_request() {
    let client = Mail::fake().from("a@b.c").client();
    let err = client
        .send(Email::new().to("not-an-email").subject("x").text("y"))
        .await
        .unwrap_err();
    assert!(matches!(err, sova_core::Error::BadRequest(_)));
}

#[tokio::test]
async fn missing_to_is_bad_request() {
    let client = Mail::fake().from("a@b.c").client();
    let err = client
        .send(Email::new().subject("x").text("y"))
        .await
        .unwrap_err();
    assert!(matches!(err, sova_core::Error::BadRequest(_)));
}

#[tokio::test]
async fn try_from_vars_invalid_url_is_err() {
    let err = match Mail::try_from_vars(|k| match k {
        "SOVA_MAIL" => Some("smtp".into()),
        "SOVA_MAIL_URL" => Some("not-a-valid-smtp-url".into()),
        _ => None,
    }) {
        Ok(_) => panic!("expected Err for invalid smtp url"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(msg.contains("mail url") || msg.contains("smtp"), "{msg}");
}

#[tokio::test]
async fn try_from_vars_missing_defaults_to_fake() {
    let mail = Mail::try_from_vars(|k| match k {
        "SOVA_MAIL_FROM" => Some("Dev <dev@localhost>".into()),
        _ => None,
    })
    .unwrap();
    assert!(mail.recorder().is_some());
}

#[tokio::test]
async fn file_transport_writes_eml() {
    let dir = tempfile::tempdir().unwrap();
    let plugin = Mail::file(dir.path()).from("App <noreply@test.local>");
    let client = plugin.client();
    client
        .send(
            Email::new()
                .to("user@example.com")
                .subject("File")
                .text("body"),
        )
        .await
        .unwrap();

    let mut found = false;
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("eml") {
            let contents = std::fs::read_to_string(&path).unwrap();
            assert!(contents.contains("Subject: File") || contents.contains("File"));
            found = true;
        }
    }
    assert!(found, "expected .eml in outbox");
}
