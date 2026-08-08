//! Markdown + Mailable smoke tests.

use sova_core::{App, Plugin, Request, Response};
use sova_mail::{Content, Envelope, Mail, MailExt, Mailable};

struct HelloMail {
    name: String,
}

impl Mailable for HelloMail {
    fn envelope(&self) -> Envelope {
        Envelope::new(format!("Hello {}", self.name))
            .from("App <a@b.c>")
            .cc("cc@example.com")
            .bcc("bcc@example.com")
    }

    fn content(&self) -> Content {
        Content::html_with_text(
            format!("<p>Hi {}</p>", self.name),
            format!("Hi {}", self.name),
        )
    }
}

struct TextOnlyMail;

impl Mailable for TextOnlyMail {
    fn envelope(&self) -> Envelope {
        Envelope::new("Plain")
    }

    fn content(&self) -> Content {
        Content::text("just text")
    }
}

#[tokio::test]
async fn mailable_send_records_html() {
    let plugin = Mail::fake().from("App <a@b.c>");
    let client = plugin.client();
    let mut app = App::new();
    plugin.install(&mut app);

    app.post("/send", |req: Request| async move {
        req.mail()
            .to("u@example.com")
            .send_mail(HelloMail {
                name: "Ada".into(),
            })
            .await?;
        Ok::<_, sova_core::Error>(Response::text("ok"))
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
    assert_eq!(sent[0].subject, "Hello Ada");
    assert_eq!(sent[0].html.as_deref(), Some("<p>Hi Ada</p>"));
    assert_eq!(sent[0].cc, vec!["cc@example.com"]);
    assert_eq!(sent[0].bcc, vec!["bcc@example.com"]);
}

#[tokio::test]
async fn mailable_text_only_and_attach_bytes() {
    let plugin = Mail::fake().from("a@b.c");
    let client = plugin.client();
    client
        .compose()
        .to("u@example.com")
        .send_mail(TextOnlyMail)
        .await
        .unwrap();
    client
        .compose()
        .to("u@example.com")
        .subject("Att")
        .text("body")
        .attach_bytes("note.txt", b"hello")
        .send()
        .await
        .unwrap();

    let sent = client.fake().unwrap().sent();
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].text.as_deref(), Some("just text"));
    assert!(sent[0].html.is_none());
    assert_eq!(sent[1].attachments, vec!["note.txt"]);
}

#[cfg(feature = "markdown")]
#[tokio::test]
async fn mailable_markdown_content() {
    struct MdMail;
    impl Mailable for MdMail {
        fn envelope(&self) -> Envelope {
            Envelope::new("Md")
        }
        fn content(&self) -> Content {
            Content::markdown("## Hello\n\n*item*")
        }
    }

    let plugin = Mail::fake().from("a@b.c");
    let client = plugin.client();
    client
        .send_mail("u@example.com", MdMail)
        .await
        .unwrap();
    let snap = &client.fake().unwrap().sent()[0];
    let html = snap.html.as_deref().unwrap();
    assert!(html.contains("<h2>Hello</h2>"), "{html}");
    assert!(snap.text.as_deref().unwrap().contains("## Hello"));
}

#[cfg(feature = "markdown")]
#[tokio::test]
async fn markdown_converts_to_html() {
    let plugin = Mail::fake().from("a@b.c");
    let client = plugin.client();
    client
        .compose()
        .to("u@example.com")
        .subject("Md")
        .markdown("# Title\n\nHello **world**")
        .send()
        .await
        .unwrap();
    let html = client.fake().unwrap().sent()[0].html.clone().unwrap();
    assert!(html.contains("<h1>Title</h1>"), "{html}");
    assert!(html.contains("<strong>world</strong>"), "{html}");
}

#[tokio::test]
async fn content_html_only_and_client_send_mail() {
    struct HtmlMail;
    impl Mailable for HtmlMail {
        fn envelope(&self) -> Envelope {
            Envelope::new("H")
        }
        fn content(&self) -> Content {
            Content::html("<b>x</b>")
        }
    }

    let plugin = Mail::fake().from("a@b.c");
    let client = plugin.client();
    client.send_mail("u@example.com", HtmlMail).await.unwrap();
    let snap = &client.fake().unwrap().sent()[0];
    assert_eq!(snap.html.as_deref(), Some("<b>x</b>"));
    assert!(snap.text.is_none());
}

#[cfg(all(feature = "templates", feature = "markdown"))]
#[tokio::test]
async fn content_markdown_view_via_mailable() {
    use sova_templates::Templates;
    use serde_json::json;
    use tempfile::tempdir;

    struct MdViewMail;
    impl Mailable for MdViewMail {
        fn envelope(&self) -> Envelope {
            Envelope::new("MV")
        }
        fn content(&self) -> Content {
            Content::markdown_view("note.md", json!({ "title": "T", "name": "N" }))
        }
    }

    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("note.md"),
        "# {{ title }}\n\n{{ name }}",
    )
    .unwrap();

    let mut app = App::new();
    app.install(Templates::minijinja(dir.path()).autoreload(false));
    let plugin = Mail::fake().from("a@b.c");
    let client = plugin.client();
    plugin.install(&mut app);

    client
        .send_mail("u@example.com", MdViewMail)
        .await
        .unwrap();
    let html = client.fake().unwrap().sent()[0].html.clone().unwrap();
    assert!(html.contains("<h1>T</h1>"), "{html}");
}
