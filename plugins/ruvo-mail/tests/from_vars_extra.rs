//! Extra Mail::try_from_vars / SmtpBuilder coverage.

use ruvo_mail::{Email, Mail};

#[test]
fn try_from_vars_unknown_mailer() {
    let err = match Mail::try_from_vars(|k| match k {
        "RUVO_MAIL" => Some("pigeon".into()),
        _ => None,
    }) {
        Ok(_) => panic!("expected unknown mailer"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("unknown"));
}

#[test]
fn try_from_vars_file_and_explicit_fake() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_string_lossy().into_owned();
    let mail = Mail::try_from_vars(|k| match k {
        "RUVO_MAIL" => Some("file".into()),
        "RUVO_MAIL_PATH" => Some(path.clone()),
        "RUVO_MAIL_FROM" => Some("File <f@t.local>".into()),
        _ => None,
    })
    .unwrap();
    assert!(mail.recorder().is_none());

    let fake = Mail::try_from_vars(|k| match k {
        "RUVO_MAIL" => Some("fake".into()),
        "RUVO_MAIL_FROM" => Some("Fake <f@t.local>".into()),
        _ => None,
    })
    .unwrap();
    assert!(fake.recorder().is_some());
}

#[test]
fn try_from_vars_smtp_requires_url() {
    let err = match Mail::try_from_vars(|k| match k {
        "RUVO_MAIL" => Some("smtp".into()),
        _ => None,
    }) {
        Ok(_) => panic!("expected missing url"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("RUVO_MAIL_URL") || err.to_string().contains("smtp"));
}

#[tokio::test]
async fn smtp_builder_builds_without_send() {
    let mail = Mail::smtp("127.0.0.1")
        .port(2525)
        .credentials("u", "p")
        .from("Smtp <s@t.local>")
        .build()
        .expect("smtp builder");
    assert!(mail.recorder().is_none());
    let client = mail.client();
    // Do not send — no local SMTP; just ensure compose works.
    let _ = client.compose().to("a@b.c").subject("x").text("y");
}

#[tokio::test]
async fn file_from_vars_sends_eml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_string_lossy().into_owned();
    let mail = Mail::try_from_vars(|k| match k {
        "RUVO_MAIL" => Some("file".into()),
        "RUVO_MAIL_PATH" => Some(path.clone()),
        "RUVO_MAIL_FROM" => Some("App <a@t.local>".into()),
        _ => None,
    })
    .unwrap();
    let client = mail.client();
    client
        .send(
            Email::new()
                .to("u@example.com")
                .subject("Vars")
                .text("body"),
        )
        .await
        .unwrap();
    let found = std::fs::read_dir(dir.path())
        .unwrap()
        .any(|e| e.unwrap().path().extension().and_then(|x| x.to_str()) == Some("eml"));
    assert!(found);
}
