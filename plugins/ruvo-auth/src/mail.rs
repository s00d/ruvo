//! Mail helpers for verify / reset.

use ruvo_core::{Request, Result};
use ruvo_mail::MailExt;

pub async fn send_verify(req: &Request, to: &str, link: &str) -> Result<()> {
    req.mail()
        .to(to)
        .subject("Verify your email")
        .text(format!("Verify your email:\n\n{link}\n"))
        .html(format!(
            "<p>Verify your email:</p><p><a href=\"{link}\">{link}</a></p>"
        ))
        .send()
        .await
}

pub async fn send_reset(req: &Request, to: &str, link: &str) -> Result<()> {
    req.mail()
        .to(to)
        .subject("Reset your password")
        .text(format!("Reset your password:\n\n{link}\n"))
        .html(format!(
            "<p>Reset your password:</p><p><a href=\"{link}\">{link}</a></p>"
        ))
        .send()
        .await
}
