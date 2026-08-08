//! Mail helpers for verify / reset ([`Mailable`](ruvo_mail::Mailable) + templates).

use ruvo_core::{Request, Result};
use ruvo_mail::{Content, Envelope, MailExt, Mailable};
#[cfg(feature = "templates")]
use serde_json::json;

/// Email-verification message.
pub struct VerifyEmailMail {
    pub link: String,
    /// Prefer MiniJinja `mail/verify.html` when Templates is installed.
    pub(crate) prefer_view: bool,
}

impl VerifyEmailMail {
    pub fn new(link: impl Into<String>) -> Self {
        Self {
            link: link.into(),
            prefer_view: true,
        }
    }
}

impl Mailable for VerifyEmailMail {
    fn envelope(&self) -> Envelope {
        Envelope::new("Verify your email")
    }

    fn content(&self) -> Content {
        let text = format!("Verify your email:\n\n{}\n", self.link);
        let html = format!(
            "<p>Verify your email:</p><p><a href=\"{0}\">{0}</a></p>",
            self.link
        );
        #[cfg(feature = "templates")]
        if self.prefer_view {
            return Content::view_with_text(
                "mail/verify.html",
                json!({ "link": self.link }),
                text,
            );
        }
        Content::html_with_text(html, text)
    }
}

/// Password-reset message.
pub struct ResetPasswordMail {
    pub link: String,
    pub(crate) prefer_view: bool,
}

impl ResetPasswordMail {
    pub fn new(link: impl Into<String>) -> Self {
        Self {
            link: link.into(),
            prefer_view: true,
        }
    }
}

impl Mailable for ResetPasswordMail {
    fn envelope(&self) -> Envelope {
        Envelope::new("Reset your password")
    }

    fn content(&self) -> Content {
        let text = format!("Reset your password:\n\n{}\n", self.link);
        let html = format!(
            "<p>Reset your password:</p><p><a href=\"{0}\">{0}</a></p>",
            self.link
        );
        #[cfg(feature = "templates")]
        if self.prefer_view {
            return Content::view_with_text(
                "mail/reset.html",
                json!({ "link": self.link }),
                text,
            );
        }
        Content::html_with_text(html, text)
    }
}

fn templates_ready(req: &Request) -> bool {
    #[cfg(feature = "templates")]
    {
        req.try_state::<ruvo_templates::MiniJinjaTemplates>()
            .is_some()
    }
    #[cfg(not(feature = "templates"))]
    {
        let _ = req;
        false
    }
}

pub async fn send_verify(req: &Request, to: &str, link: &str) -> Result<()> {
    let mut mail = VerifyEmailMail::new(link);
    mail.prefer_view = templates_ready(req);
    req.mail().to(to).send_mail(mail).await
}

pub async fn send_reset(req: &Request, to: &str, link: &str) -> Result<()> {
    let mut mail = ResetPasswordMail::new(link);
    mail.prefer_view = templates_ready(req);
    req.mail().to(to).send_mail(mail).await
}
