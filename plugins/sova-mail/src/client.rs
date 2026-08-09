//! [`Mail`] plugin builder and [`MailClient`] transport handle.

use crate::email::Email;
use crate::fake::FakeMail;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::{AsyncFileTransport, AsyncTransport, Tokio1Executor};
use sova_core::{Error, Result};
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "templates")]
type TemplatesSlot =
    Arc<std::sync::RwLock<Option<Arc<sova_templates::MiniJinjaTemplates>>>>;

enum Backend {
    Smtp(AsyncSmtpTransport<Tokio1Executor>),
    File(AsyncFileTransport<Tokio1Executor>),
    Fake(FakeMail),
}

/// Plugin installer — call [`Self::client`] before `app.install` to share the handle (e.g. tasks).
pub struct Mail {
    backend: Backend,
    default_from: Option<String>,
    /// True when `.from()` / env set an address (toml must not overwrite).
    from_explicit: bool,
    fake: Option<FakeMail>,
    #[cfg(feature = "templates")]
    templates_slot: TemplatesSlot,
}

impl Mail {
    fn bare(backend: Backend, default_from: Option<String>, fake: Option<FakeMail>) -> Self {
        Self {
            backend,
            default_from,
            from_explicit: false,
            fake,
            #[cfg(feature = "templates")]
            templates_slot: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    pub(crate) fn is_from_explicit(&self) -> bool {
        self.from_explicit
    }

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
        Self::bare(
            Backend::Fake(fake.clone()),
            Some("Sova <noreply@localhost>".into()),
            Some(fake),
        )
    }

    /// Write `.eml` files under `dir`.
    pub fn file(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let transport = AsyncFileTransport::<Tokio1Executor>::new(dir);
        Self::bare(
            Backend::File(transport),
            Some("Sova <noreply@localhost>".into()),
            None,
        )
    }

    /// Like [`Self::try_from_env`], panics on invalid SMTP URL / unknown mailer.
    ///
    /// Missing mailer + missing URL still yields fake (dev DX). Prefer
    /// [`Self::try_from_env`] in `main` when you want `?`.
    pub fn from_env() -> Self {
        Self::try_from_env().unwrap_or_else(|e| {
            panic!("mail: from_env failed: {e}");
        })
    }

    /// Build from process env — see [`Self::try_from_vars`].
    pub fn try_from_env() -> Result<Self> {
        Self::try_from_vars(|k| std::env::var(k).ok().filter(|s| !s.is_empty()))
    }

    /// Build from a lookup fn (tests / custom env sources).
    ///
    /// - `SOVA_MAIL` / `SOVA_MAIL_MAILER`: `fake` | `smtp` | `file` (optional)
    /// - No mailer + no URL → [`Self::fake`] (dev)
    /// - `smtp` / URL set + parse failure → **`Err`** (never silently fakes)
    /// - `file` → `SOVA_MAIL_PATH` (default `./mail`)
    pub fn try_from_vars<F>(mut get: F) -> Result<Self>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let from = get("SOVA_MAIL_FROM");
        let mailer = get("SOVA_MAIL")
            .or_else(|| get("SOVA_MAIL_MAILER"))
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty());
        let url = get("SOVA_MAIL_URL").or_else(|| get("SMTP_URL"));

        let kind = match mailer.as_deref() {
            Some("fake") => "fake",
            Some("file") => "file",
            Some("smtp") => "smtp",
            Some(other) => {
                return Err(Error::Internal(format!(
                    "unknown SOVA_MAIL={other} (use fake|smtp|file)"
                )));
            }
            None if url.is_some() => "smtp",
            None => "fake",
        };

        match kind {
            "fake" => {
                if mailer.is_none() {
                    tracing::info!("mail: no SOVA_MAIL / SOVA_MAIL_URL — using fake transport");
                }
                let mut m = Self::fake();
                if let Some(f) = from {
                    m.default_from = Some(f);
                    m.from_explicit = true;
                }
                Ok(m)
            }
            "file" => {
                let path = get("SOVA_MAIL_PATH").unwrap_or_else(|| "./mail".into());
                let mut m = Self::file(path);
                if let Some(f) = from {
                    m.default_from = Some(f);
                    m.from_explicit = true;
                }
                Ok(m)
            }
            "smtp" => {
                let url = url.ok_or_else(|| {
                    Error::Internal("SOVA_MAIL=smtp requires SOVA_MAIL_URL or SMTP_URL".into())
                })?;
                let transport = build_smtp_from_url(&url)?;
                let mut m = Self::bare(
                    Backend::Smtp(transport),
                    from.clone().or_else(|| Some("Sova <noreply@localhost>".into())),
                    None,
                );
                if from.is_some() {
                    m.from_explicit = true;
                }
                Ok(m)
            }
            _ => unreachable!(),
        }
    }

    pub fn from(mut self, addr: impl Into<String>) -> Self {
        self.default_from = Some(addr.into());
        self.from_explicit = true;
        self
    }

    /// Shared client (clone before `install` for background jobs).
    ///
    /// With feature `templates`, the templates slot is shared with the installed
    /// client — wiring at install / startup reaches earlier clones too.
    pub fn client(&self) -> MailClient {
        MailClient {
            backend: match &self.backend {
                Backend::Smtp(t) => Arc::new(ClientBackend::Smtp(t.clone())),
                Backend::File(t) => Arc::new(ClientBackend::File(t.clone())),
                Backend::Fake(f) => Arc::new(ClientBackend::Fake(f.clone())),
            },
            default_from: self.default_from.clone(),
            fake: self.fake.clone(),
            events: None,
            #[cfg(feature = "templates")]
            templates: Arc::clone(&self.templates_slot),
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
            panic!("mail smtp build failed (refusing silent fake): {e}");
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
        let explicit = self.from.is_some();
        let mut mail = Mail::bare(
            Backend::Smtp(transport),
            self.from
                .or_else(|| Some("Sova <noreply@localhost>".into())),
            None,
        );
        mail.from_explicit = explicit;
        Ok(mail)
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
    events: Option<sova_core::EventBus>,
    #[cfg(feature = "templates")]
    templates: TemplatesSlot,
}

impl MailClient {
    pub fn compose(&self) -> Email {
        Email::with_client(self.clone())
    }

    /// Soft-wire [`EventBus`] for [`crate::MailSent`] emits after send.
    pub fn set_events(&mut self, bus: sova_core::EventBus) {
        self.events = Some(bus);
    }

    #[cfg(feature = "templates")]
    pub(crate) fn templates(&self) -> Option<Arc<sova_templates::MiniJinjaTemplates>> {
        self.templates.read().unwrap().clone()
    }

    #[cfg(feature = "templates")]
    pub(crate) fn set_templates(&self, templates: sova_templates::MiniJinjaTemplates) {
        *self.templates.write().unwrap() = Some(Arc::new(templates));
    }

    #[allow(unused_mut)] // mut needed when resolving deferred view/markdown bodies
    pub async fn send(&self, mut email: Email) -> Result<()> {
        #[cfg(any(feature = "templates", feature = "markdown"))]
        email.resolve_body(self)?;
        let (snap, message) = email.into_message(self.default_from.as_deref())?;
        let to = snap.to.clone();
        let subject = snap.subject.clone();
        let result = match self.backend.as_ref() {
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
        };
        if result.is_ok() {
            if let Some(bus) = &self.events {
                bus.dispatch(crate::MailSent { to, subject });
            }
        }
        result
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
