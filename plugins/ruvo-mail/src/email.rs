//! Fluent email builder → `lettre::Message`.

use crate::client::MailClient;
use lettre::message::{header::ContentType, Attachment as LAttach, MultiPart, SinglePart};
use lettre::Message;
use ruvo_core::{Error, Result};
use std::path::PathBuf;

/// Built message metadata (fake transport / asserts).
#[derive(Clone, Debug)]
pub struct EmailSnapshot {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub attachments: Vec<String>,
}

/// Fluent outbound message (Nodemailer-style `sendMail` fields).
#[derive(Clone)]
pub struct Email {
    client: Option<MailClient>,
    from: Option<String>,
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: String,
    text: Option<String>,
    html: Option<String>,
    attachments: Vec<Attachment>,
}

#[derive(Clone)]
enum Attachment {
    Path(PathBuf),
    Bytes { filename: String, data: Vec<u8> },
}

impl Email {
    pub fn new() -> Self {
        Self {
            client: None,
            from: None,
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: String::new(),
            text: None,
            html: None,
            attachments: Vec::new(),
        }
    }

    pub(crate) fn with_client(client: MailClient) -> Self {
        let from = client.default_from.clone();
        Self {
            client: Some(client),
            from,
            ..Self::new()
        }
    }

    pub fn from(mut self, addr: impl Into<String>) -> Self {
        self.from = Some(addr.into());
        self
    }

    pub fn to(mut self, addr: impl Into<String>) -> Self {
        self.to.push(addr.into());
        self
    }

    pub fn cc(mut self, addr: impl Into<String>) -> Self {
        self.cc.push(addr.into());
        self
    }

    pub fn bcc(mut self, addr: impl Into<String>) -> Self {
        self.bcc.push(addr.into());
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn text(mut self, body: impl Into<String>) -> Self {
        self.text = Some(body.into());
        self
    }

    pub fn html(mut self, body: impl Into<String>) -> Self {
        self.html = Some(body.into());
        self
    }

    pub fn attach(mut self, path: impl Into<PathBuf>) -> Self {
        self.attachments.push(Attachment::Path(path.into()));
        self
    }

    pub fn attach_bytes(mut self, filename: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        self.attachments.push(Attachment::Bytes {
            filename: filename.into(),
            data: data.into(),
        });
        self
    }

    /// Send via the client bound by [`MailExt::mail`] / [`MailClient::compose`].
    pub async fn send(self) -> Result<()> {
        let client = self
            .client
            .clone()
            .ok_or_else(|| Error::Internal("mail: no client — use MailClient::send".into()))?;
        client.send(self).await
    }

    pub(crate) fn snapshot(&self) -> EmailSnapshot {
        EmailSnapshot {
            from: self.from.clone().unwrap_or_default(),
            to: self.to.clone(),
            cc: self.cc.clone(),
            bcc: self.bcc.clone(),
            subject: self.subject.clone(),
            text: self.text.clone(),
            html: self.html.clone(),
            attachments: self
                .attachments
                .iter()
                .map(|a| match a {
                    Attachment::Path(p) => p.display().to_string(),
                    Attachment::Bytes { filename, .. } => filename.clone(),
                })
                .collect(),
        }
    }

    pub(crate) fn into_message(self, default_from: Option<&str>) -> Result<(EmailSnapshot, Message)> {
        if self.to.is_empty() {
            return Err(Error::BadRequest("mail: at least one `to` required".into()));
        }
        let from = self
            .from
            .as_deref()
            .or(default_from)
            .ok_or_else(|| Error::BadRequest("mail: `from` required".into()))?
            .to_string();

        let mut snap = self.snapshot();
        snap.from = from.clone();

        let mut builder = Message::builder().from(
            from.parse()
                .map_err(|e| Error::BadRequest(format!("mail from: {e}")))?,
        );
        for addr in &self.to {
            builder = builder.to(addr
                .parse()
                .map_err(|e| Error::BadRequest(format!("mail to: {e}")))?);
        }
        for addr in &self.cc {
            builder = builder.cc(addr
                .parse()
                .map_err(|e| Error::BadRequest(format!("mail cc: {e}")))?);
        }
        for addr in &self.bcc {
            builder = builder.bcc(addr
                .parse()
                .map_err(|e| Error::BadRequest(format!("mail bcc: {e}")))?);
        }
        builder = builder.subject(&self.subject);

        let content = match (&self.text, &self.html) {
            (Some(text), Some(html)) => MultiPart::alternative()
                .singlepart(plain(text))
                .singlepart(html_part(html)),
            (Some(text), None) => MultiPart::mixed().singlepart(plain(text)),
            (None, Some(html)) => MultiPart::mixed().singlepart(html_part(html)),
            (None, None) => MultiPart::mixed().singlepart(plain("")),
        };

        let message = if self.attachments.is_empty() {
            builder
                .multipart(content)
                .map_err(|e| Error::BadRequest(format!("mail build: {e}")))?
        } else {
            let mut mixed = MultiPart::mixed().multipart(content);
            for att in &self.attachments {
                let (filename, data) = match att {
                    Attachment::Path(path) => {
                        let filename = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("attachment")
                            .to_string();
                        let data = std::fs::read(path).map_err(|e| {
                            Error::Internal(format!("mail attach {}: {e}", path.display()))
                        })?;
                        (filename, data)
                    }
                    Attachment::Bytes { filename, data } => (filename.clone(), data.clone()),
                };
                let ct = content_type_for(&filename);
                mixed = mixed.singlepart(LAttach::new(filename).body(data, ct));
            }
            builder
                .multipart(mixed)
                .map_err(|e| Error::BadRequest(format!("mail build: {e}")))?
        };

        Ok((snap, message))
    }
}

impl Default for Email {
    fn default() -> Self {
        Self::new()
    }
}

fn plain(body: &str) -> SinglePart {
    SinglePart::builder()
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
}

fn html_part(body: &str) -> SinglePart {
    SinglePart::builder()
        .header(ContentType::TEXT_HTML)
        .body(body.to_string())
}

fn content_type_for(filename: &str) -> ContentType {
    let mime = mime_guess_lite(filename);
    mime.parse::<ContentType>().unwrap_or(ContentType::TEXT_PLAIN)
}

fn mime_guess_lite(filename: &str) -> &'static str {
    match filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}
