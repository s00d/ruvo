use crate::collector::{DevToolsBag, MailLine};

pub fn collect_mail(bag: &DevToolsBag, mail: Option<&sova_mail::MailClient>) {
    if let Some(client) = mail {
        if let Some(fake) = client.fake() {
            for m in fake.sent().into_iter().rev().take(5) {
                bag.push_mail(MailLine {
                    to: m.to,
                    subject: m.subject,
                    backend: "fake".into(),
                });
            }
        }
    }
}
