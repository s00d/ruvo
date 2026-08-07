//! Outbound email for Ruvo (Express/Nodemailer-simple API on [lettre](https://lettre.rs/)).

mod client;
mod email;
mod fake;

pub use client::{Mail, MailClient, SmtpBuilder};
pub use email::{Email, EmailSnapshot};
pub use fake::FakeMail;

use ruvo_core::{App, Plugin, Request};

/// `req.mail()` — start an [`Email`] with the installed client's default `From`.
pub trait MailExt {
    fn mail(&self) -> Email;
}

impl MailExt for Request {
    fn mail(&self) -> Email {
        self.state::<MailClient>().compose()
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
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("mail") {
                if let Some(from) = section.get("from").and_then(|v| v.as_str()) {
                    mail = mail.from(from);
                }
            }
        }
        app.state(mail.into_client());
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
        Mail::from_smtp(self).install(app);
    }
}
