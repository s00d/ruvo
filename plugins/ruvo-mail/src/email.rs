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

#[cfg(feature = "templates")]
#[derive(Clone)]
struct PendingView {
    name: String,
    ctx: serde_json::Value,
}

/// Fluent outbound message (Nodemailer / Laravel-style).
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
    #[cfg(feature = "templates")]
    html_view: Option<PendingView>,
    #[cfg(feature = "templates")]
    text_view: Option<PendingView>,
    #[cfg(feature = "templates")]
    ambient: Option<ruvo_templates::FrozenAmbient>,
    #[cfg(feature = "markdown")]
    pending_markdown: Option<String>,
    #[cfg(all(feature = "templates", feature = "markdown"))]
    markdown_view: Option<PendingView>,
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
            #[cfg(feature = "templates")]
            html_view: None,
            #[cfg(feature = "templates")]
            text_view: None,
            #[cfg(feature = "templates")]
            ambient: None,
            #[cfg(feature = "markdown")]
            pending_markdown: None,
            #[cfg(all(feature = "templates", feature = "markdown"))]
            markdown_view: None,
        }
    }

    pub(crate) fn with_client(client: MailClient) -> Self {
        let from = client.default_from.clone();
        #[cfg(feature = "templates")]
        let ambient = client.templates().map(|t| t.freeze_globals());
        Self {
            client: Some(client),
            from,
            #[cfg(feature = "templates")]
            ambient,
            ..Self::new()
        }
    }

    #[cfg(feature = "templates")]
    pub(crate) fn with_ambient(mut self, ambient: ruvo_templates::FrozenAmbient) -> Self {
        self.ambient = Some(ambient);
        self
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
        #[cfg(feature = "templates")]
        {
            self.text_view = None;
        }
        self
    }

    pub fn html(mut self, body: impl Into<String>) -> Self {
        self.html = Some(body.into());
        #[cfg(feature = "templates")]
        {
            self.html_view = None;
        }
        #[cfg(feature = "markdown")]
        {
            self.pending_markdown = None;
        }
        #[cfg(all(feature = "templates", feature = "markdown"))]
        {
            self.markdown_view = None;
        }
        self
    }

    /// Defer MiniJinja HTML render until [`Self::send`] (Laravel-style `view`).
    ///
    /// Layouts: `{% extends "mail/layout.html" %}` in the template file.
    /// Requires feature `templates` and an installed [`ruvo_templates::MiniJinjaTemplates`]
    /// wired onto the [`MailClient`] (Templates → Mail install order, or startup hook).
    #[cfg(feature = "templates")]
    pub fn view<T: serde::Serialize>(mut self, name: impl Into<String>, ctx: T) -> Self {
        let ctx = serde_json::to_value(ctx).unwrap_or(serde_json::Value::Null);
        self.html_view = Some(PendingView {
            name: name.into(),
            ctx,
        });
        self.html = None;
        #[cfg(feature = "markdown")]
        {
            self.pending_markdown = None;
        }
        #[cfg(all(feature = "templates", feature = "markdown"))]
        {
            self.markdown_view = None;
        }
        self
    }

    /// Like [`Self::view`], but for the plain-text body.
    #[cfg(feature = "templates")]
    pub fn text_view<T: serde::Serialize>(mut self, name: impl Into<String>, ctx: T) -> Self {
        let ctx = serde_json::to_value(ctx).unwrap_or(serde_json::Value::Null);
        self.text_view = Some(PendingView {
            name: name.into(),
            ctx,
        });
        self.text = None;
        self
    }

    /// Defer markdown→HTML until [`Self::send`] (feature `markdown`).
    ///
    /// Also sets the plain-text body to the raw markdown source (unless you call
    /// [`Self::text`] afterwards).
    #[cfg(feature = "markdown")]
    pub fn markdown(mut self, md: impl Into<String>) -> Self {
        let md = md.into();
        self.text = Some(md.clone());
        self.pending_markdown = Some(md);
        self.html = None;
        #[cfg(feature = "templates")]
        {
            self.html_view = None;
            self.markdown_view = None;
        }
        self
    }

    /// Render a MiniJinja template as markdown, then convert to HTML at send.
    #[cfg(all(feature = "templates", feature = "markdown"))]
    pub fn markdown_view<T: serde::Serialize>(mut self, name: impl Into<String>, ctx: T) -> Self {
        let ctx = serde_json::to_value(ctx).unwrap_or(serde_json::Value::Null);
        self.markdown_view = Some(PendingView {
            name: name.into(),
            ctx,
        });
        self.html = None;
        self.html_view = None;
        self.pending_markdown = None;
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

    #[cfg(any(feature = "templates", feature = "markdown"))]
    pub(crate) fn resolve_body(&mut self, client: &MailClient) -> Result<()> {
        #[cfg(feature = "templates")]
        {
            let needs_templates = self.html_view.is_some()
                || self.text_view.is_some()
                || {
                    #[cfg(feature = "markdown")]
                    {
                        self.markdown_view.is_some()
                    }
                    #[cfg(not(feature = "markdown"))]
                    {
                        false
                    }
                };
            if needs_templates {
                let templates = client.templates().ok_or_else(|| {
                    Error::Internal(
                        "mail view requires Templates plugin (install Templates before Mail, or enable mail-templates)"
                            .into(),
                    )
                })?;
                let ambient = self
                    .ambient
                    .clone()
                    .unwrap_or_else(|| templates.freeze_globals());
                if let Some(view) = self.html_view.take() {
                    self.html = Some(templates.render_owned(&ambient, &view.name, view.ctx)?);
                }
                if let Some(view) = self.text_view.take() {
                    self.text = Some(templates.render_owned(&ambient, &view.name, view.ctx)?);
                }
                #[cfg(feature = "markdown")]
                if let Some(view) = self.markdown_view.take() {
                    let md = templates.render_owned(&ambient, &view.name, view.ctx)?;
                    if self.text.is_none() {
                        self.text = Some(md.clone());
                    }
                    self.html = Some(crate::markdown::to_html(&md));
                }
                let _ = self.ambient.take();
            }
        }

        #[cfg(feature = "markdown")]
        if let Some(md) = self.pending_markdown.take() {
            self.html = Some(crate::markdown::to_html(&md));
        }

        Ok(())
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
