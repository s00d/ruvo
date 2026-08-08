//! Laravel-style [`Mailable`] — envelope + content applied onto [`Email`].

use crate::email::Email;
use ruvo_core::Result;

/// Message metadata (subject / from / cc / bcc). Recipients stay on [`Email::to`].
#[derive(Clone, Debug, Default)]
pub struct Envelope {
    pub subject: String,
    pub from: Option<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
}

impl Envelope {
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            ..Default::default()
        }
    }

    pub fn from(mut self, addr: impl Into<String>) -> Self {
        self.from = Some(addr.into());
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
}

#[derive(Clone)]
enum ContentInner {
    Html {
        html: String,
        text: Option<String>,
    },
    Text(String),
    #[cfg(feature = "templates")]
    View {
        name: String,
        ctx: serde_json::Value,
        text: Option<String>,
    },
    #[cfg(feature = "markdown")]
    Markdown {
        md: String,
        /// When true, also set plain-text body to the raw markdown source.
        with_text: bool,
    },
    #[cfg(all(feature = "templates", feature = "markdown"))]
    MarkdownView {
        name: String,
        ctx: serde_json::Value,
        with_text: bool,
    },
}

/// Body of a [`Mailable`] (html / text / view / markdown).
#[derive(Clone)]
pub struct Content {
    inner: ContentInner,
}

impl Content {
    pub fn html(html: impl Into<String>) -> Self {
        Self {
            inner: ContentInner::Html {
                html: html.into(),
                text: None,
            },
        }
    }

    pub fn html_with_text(html: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            inner: ContentInner::Html {
                html: html.into(),
                text: Some(text.into()),
            },
        }
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self {
            inner: ContentInner::Text(text.into()),
        }
    }

    /// MiniJinja HTML view (feature `templates`).
    #[cfg(feature = "templates")]
    pub fn view<T: serde::Serialize>(name: impl Into<String>, ctx: T) -> Self {
        let ctx = serde_json::to_value(ctx).unwrap_or(serde_json::Value::Null);
        Self {
            inner: ContentInner::View {
                name: name.into(),
                ctx,
                text: None,
            },
        }
    }

    /// Like [`Self::view`], plus a plain-text alternative.
    #[cfg(feature = "templates")]
    pub fn view_with_text<T: serde::Serialize>(
        name: impl Into<String>,
        ctx: T,
        text: impl Into<String>,
    ) -> Self {
        let ctx = serde_json::to_value(ctx).unwrap_or(serde_json::Value::Null);
        Self {
            inner: ContentInner::View {
                name: name.into(),
                ctx,
                text: Some(text.into()),
            },
        }
    }

    /// Raw markdown → HTML at send (feature `markdown`).
    #[cfg(feature = "markdown")]
    pub fn markdown(md: impl Into<String>) -> Self {
        Self {
            inner: ContentInner::Markdown {
                md: md.into(),
                with_text: true,
            },
        }
    }

    /// MiniJinja template whose output is markdown, then converted to HTML.
    #[cfg(all(feature = "templates", feature = "markdown"))]
    pub fn markdown_view<T: serde::Serialize>(name: impl Into<String>, ctx: T) -> Self {
        let ctx = serde_json::to_value(ctx).unwrap_or(serde_json::Value::Null);
        Self {
            inner: ContentInner::MarkdownView {
                name: name.into(),
                ctx,
                with_text: true,
            },
        }
    }

    pub(crate) fn apply(self, mut email: Email) -> Email {
        match self.inner {
            ContentInner::Html { html, text } => {
                email = email.html(html);
                if let Some(t) = text {
                    email = email.text(t);
                }
                email
            }
            ContentInner::Text(text) => email.text(text),
            #[cfg(feature = "templates")]
            ContentInner::View { name, ctx, text } => {
                email = email.view(name, ctx);
                if let Some(t) = text {
                    email = email.text(t);
                }
                email
            }
            #[cfg(feature = "markdown")]
            ContentInner::Markdown { md, with_text } => {
                email = email.markdown(md);
                if !with_text {
                    // markdown() also sets text; clear if caller wanted HTML-only
                }
                let _ = with_text;
                email
            }
            #[cfg(all(feature = "templates", feature = "markdown"))]
            ContentInner::MarkdownView {
                name,
                ctx,
                with_text,
            } => {
                let _ = with_text;
                email.markdown_view(name, ctx)
            }
        }
    }
}

/// Laravel-style mailable: describe envelope + content, send via [`Email::send_mail`].
pub trait Mailable: Send + Sync {
    fn envelope(&self) -> Envelope;
    fn content(&self) -> Content;

    /// Apply this mailable onto a started [`Email`] (already has `to` / client).
    fn build(&self, mut email: Email) -> Email {
        let env = self.envelope();
        email = email.subject(env.subject);
        if let Some(from) = env.from {
            email = email.from(from);
        }
        for addr in env.cc {
            email = email.cc(addr);
        }
        for addr in env.bcc {
            email = email.bcc(addr);
        }
        self.content().apply(email)
    }
}

impl Email {
    /// Apply a [`Mailable`] and [`Self::send`].
    pub async fn send_mail<M: Mailable>(self, mail: M) -> Result<()> {
        mail.build(self).send().await
    }
}

impl crate::client::MailClient {
    /// `compose().to(to).send_mail(mail)`.
    pub async fn send_mail<M: Mailable>(
        &self,
        to: impl Into<String>,
        mail: M,
    ) -> Result<()> {
        self.compose().to(to).send_mail(mail).await
    }
}
