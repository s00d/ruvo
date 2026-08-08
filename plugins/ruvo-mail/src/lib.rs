//! Outbound email for Ruvo (Express/Nodemailer-simple API on [lettre](https://lettre.rs/)).
//!
//! With feature `templates`, render MiniJinja views into the body (Laravel-style):
//!
//! ```ignore
//! req.mail()
//!     .to(user)
//!     .subject("Welcome")
//!     .view("mail/welcome.html", json!({ "name": name }))
//!     .send()
//!     .await?;
//!
//! // Mailable
//! req.mail().to(user).send_mail(WelcomeMail { name }).await?;
//!
//! // Markdown body (feature `markdown`)
//! req.mail().to(user).subject("Hi").markdown("# Hello\n\nWorld").send().await?;
//! ```
//!
//! Layouts use Jinja `{% extends "mail/layout.html" %}` in the template file.

mod client;
mod email;
mod fake;
mod mailable;

#[cfg(feature = "markdown")]
mod markdown;

pub use client::{Mail, MailClient, SmtpBuilder};
pub use email::{Email, EmailSnapshot};
pub use fake::FakeMail;
pub use mailable::{Content, Envelope, Mailable};

use ruvo_core::{App, Plugin, Request};

/// `req.mail()` — start an [`Email`] with the installed client's default `From`.
pub trait MailExt {
    fn mail(&self) -> Email;
}

impl MailExt for Request {
    fn mail(&self) -> Email {
        let email = self.state::<MailClient>().compose();
        #[cfg(feature = "templates")]
        {
            if let Some(templates) = self.try_state::<ruvo_templates::MiniJinjaTemplates>() {
                return email.with_ambient(templates.freeze_ambient(self));
            }
        }
        email
    }
}

impl Plugin for Mail {
    fn id(&self) -> &'static str {
        "mail"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Mail")
            .description("Outbound email via lettre (SMTP / fake / file)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        let mut mail = self;
        // Unset-fill from `[mail]` — explicit `.from()` / env wins.
        if !mail.is_from_explicit() {
            if let Some(doc) = app.config_doc() {
                if let Some(section) = doc.section("mail") {
                    if let Some(from) = section.get("from").and_then(|v| v.as_str()) {
                        mail = mail.from(from);
                    }
                }
            }
        }
        let client = mail.into_client();

        #[cfg(feature = "templates")]
        {
            if let Some(t) = app.try_state::<ruvo_templates::MiniJinjaTemplates>() {
                client.set_templates(t.as_ref().clone());
            }
            // If Templates is installed after Mail, pick it up before accept.
            let wire = client.clone();
            app.on_startup(move |state| {
                let wire = wire.clone();
                async move {
                    if wire.templates().is_none() {
                        if let Some(t) = state.get::<ruvo_templates::MiniJinjaTemplates>() {
                            wire.set_templates(t.as_ref().clone());
                        }
                    }
                    Ok(())
                }
            });
        }

        app.state(client);
    }
}

impl Plugin for client::SmtpBuilder {
    fn id(&self) -> &'static str {
        "mail"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Mail")
            .description("Outbound email via lettre (SMTP / fake / file)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        match self.build() {
            Ok(mail) => mail.install(app),
            Err(e) => panic!("mail smtp install failed (refusing silent fake): {e}"),
        }
    }
}
