//! [`Mail`] plugin builder and [`MailClient`] transport handle.

use crate::email::Email;
use crate::fake::FakeMail;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::{AsyncFileTransport, AsyncTransport, Tokio1Executor};
use ruvo_core::{Error, Result};
use std::path::PathBuf;
use std::sync::Arc;

enum Backend {
    Smtp(AsyncSmtpTransport<Tokio1Executor>),
    File(AsyncFileTransport<Tokio1Executor>),
    Fake(FakeMail),
}

/// Plugin installer — call [`Self::client`] before `app.install` to share the handle (e.g. tasks).
pub struct Mail {
    backend: Backend,
    default_from: Option<String>,
    fake: Option<FakeMail>,
}

impl Mail {
    /// SMTP relay (STARTTLS via lettre `relay`). Use [`SmtpBuilder::build`] or install via [`From`].
    pub fn smtp(host: impl Into<String>) -> SmtpBuilder {
        SmtpBuilder {
            host: host.into(),
            port: None,
            user: None,
            pass: None,
            from: None,
        }
    }

    /// Record messages in memory (tests / local demo).
    pub fn fake() -> Self {
        let fake = FakeMail::new();
        Self {
            backend: Backend::Fake(fake.clone()),
            default_from: Some("Ruvo <noreply@localhost>".into()),
            fake: Some(fake),
        }
    }

    /// Write `.eml` files under `dir`.
    pub fn file(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let transport = AsyncFileTransport::<Tokio1Executor>::new(dir);
        Self {
            backend: Backend::File(transport),
            default_from: Some("Ruvo <noreply@localhost>".into()),
            fake: None,
        }
    }

    /// From `RUVO_MAIL_URL` / `SMTP_URL` + `RUVO_MAIL_FROM`. Falls back to [`Self::fake`] if unset.
    pub fn from_env() -> Self {
        let from = std::env::var("RUVO_MAIL_FROM").ok();
        let url = std::env::var("RUVO_MAIL_URL")
            .or_else(|_| std::env::var("SMTP_URL"))
            .ok();

        match url {
            Some(url) => match build_smtp_from_url(&url) {
                Ok(transport) => Self {
                    backend: Backend::Smtp(transport),
                    default_from: from.or_else(|| Some("Ruvo <noreply@localhost>".into())),
                    fake: None,
                },
                Err(err) => {
                    tracing::warn!(%err, "mail: invalid RUVO_MAIL_URL / SMTP_URL — using fake");
                    let mut m = Self::fake();
                    if let Some(f) = from {
                        m.default_from = Some(f);
                    }
                    m
                }
            },
            None => {
                let mut m = Self::fake();
                if let Some(f) = from {
                    m.default_from = Some(f);
                }
                m
            }
        }
    }

    pub fn from(mut self, addr: impl Into<String>) -> Self {
        self.default_from = Some(addr.into());
        self
    }

    /// Shared client (clone before `install` for background jobs).
    pub fn client(&self) -> MailClient {
        MailClient {
            backend: match &self.backend {
                Backend::Smtp(t) => Arc::new(ClientBackend::Smtp(t.clone())),
                Backend::File(t) => Arc::new(ClientBackend::File(t.clone())),
                Backend::Fake(f) => Arc::new(ClientBackend::Fake(f.clone())),
            },
            default_from: self.default_from.clone(),
            fake: self.fake.clone(),
        }
    }

    pub(crate) fn into_client(self) -> MailClient {
        self.client()
    }

    /// Recorder when using [`Self::fake`].
    pub fn recorder(&self) -> Option<&FakeMail> {
        self.fake.as_ref()
    }

    pub(crate) fn from_smtp(b: SmtpBuilder) -> Self {
        b.build().unwrap_or_else(|e| {
            tracing::error!(error = %e, "mail smtp build failed — falling back to fake");
            Mail::fake().from(
                std::env::var("RUVO_MAIL_FROM").unwrap_or_else(|_| "Ruvo <noreply@localhost>".into()),
            )
        })
    }
}

/// Fluent SMTP configuration → [`Mail`].
pub struct SmtpBuilder {
    host: String,
    port: Option<u16>,
    user: Option<String>,
    pass: Option<String>,
    from: Option<String>,
}

impl SmtpBuilder {
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn credentials(mut self, user: impl Into<String>, pass: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self.pass = Some(pass.into());
        self
    }

    pub fn from(mut self, addr: impl Into<String>) -> Self {
        self.from = Some(addr.into());
        self
    }

    pub fn build(self) -> Result<Mail> {
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.host)
            .map_err(|e| Error::Internal(format!("mail smtp relay: {e}")))?;
        if let Some(port) = self.port {
            builder = builder.port(port);
        }
        if let (Some(user), Some(pass)) = (self.user.clone(), self.pass.clone()) {
            builder = builder.credentials(Credentials::new(user, pass));
        }
        let transport = builder.build();
        Ok(Mail {
            backend: Backend::Smtp(transport),
            default_from: self
                .from
                .or_else(|| Some("Ruvo <noreply@localhost>".into())),
            fake: None,
        })
    }
}

impl From<SmtpBuilder> for Mail {
    fn from(b: SmtpBuilder) -> Self {
        Mail::from_smtp(b)
    }
}

enum ClientBackend {
    Smtp(AsyncSmtpTransport<Tokio1Executor>),
    File(AsyncFileTransport<Tokio1Executor>),
    Fake(FakeMail),
}

/// Shared outbound mail handle (`app.state` / clone into tasks).
#[derive(Clone)]
pub struct MailClient {
    backend: Arc<ClientBackend>,
    pub(crate) default_from: Option<String>,
    fake: Option<FakeMail>,
}

impl MailClient {
    pub fn compose(&self) -> Email {
        Email::with_client(self.clone())
    }

    pub async fn send(&self, email: Email) -> Result<()> {
        let (snap, message) = email.into_message(self.default_from.as_deref())?;
        match self.backend.as_ref() {
            ClientBackend::Fake(fake) => {
                fake.record(snap);
                Ok(())
            }
            ClientBackend::Smtp(t) => t
                .send(message)
                .await
                .map(|_| ())
                .map_err(|e| Error::Internal(format!("mail smtp: {e}"))),
            ClientBackend::File(t) => t
                .send(message)
                .await
                .map(|_| ())
                .map_err(|e| Error::Internal(format!("mail file: {e}"))),
        }
    }

    /// Fake inbox when installed via [`Mail::fake`].
    pub fn fake(&self) -> Option<&FakeMail> {
        self.fake.as_ref()
    }
}

fn build_smtp_from_url(url: &str) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
    Ok(AsyncSmtpTransport::<Tokio1Executor>::from_url(url)
        .map_err(|e| Error::Internal(format!("mail url: {e}")))?
        .build())
}
