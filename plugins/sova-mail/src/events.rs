//! Domain events for outbound mail.

use sova_core::Event;

/// Fired after a message is accepted by the transport (fake/smtp/file).
#[derive(Debug, Clone)]
pub struct MailSent {
    pub to: Vec<String>,
    pub subject: String,
}

impl Event for MailSent {
    fn name(&self) -> &'static str {
        "mail.sent"
    }
}
