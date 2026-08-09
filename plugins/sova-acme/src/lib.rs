//! Let's Encrypt ACME (HTTP-01) with TLS hot-reload for Sova.
//!
//! ```ignore
//! use sova::{Acme, App, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let acme = Acme::lets_encrypt(["example.com"])
//!         .email("ops@example.com")
//!         .dir("./data/acme");
//!     let tls = acme.tls()?;
//!     let mut app = App::new();
//!     app.get("/", || async { "hello https" });
//!     app.install(acme.with_tls(tls.clone()));
//!     app.bind("0.0.0.0:443").tls(tls.hsts(true))?.run().await
//! }
//! ```

mod events;
mod handle;
mod http01;
mod issue;
mod service;
mod storage;

pub use events::{AcmeFailed, CertificateIssued, CertificateRenewed};
pub use handle::{AcmeHandle, AcmeStatus};

use http01::ChallengeMap;
use service::AcmeService;
use sova_core::{App, Plugin, PluginMeta, Result, Tls};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use storage::AcmeStorage;
use tokio::sync::Notify;

/// Let's Encrypt / ACME plugin (HTTP-01 on port 80 + background renewer).
pub struct Acme {
    domains: Vec<String>,
    email: Option<String>,
    dir: PathBuf,
    staging: bool,
    http_port: u16,
    https_port: u16,
    redirect_https: bool,
    renew_days: u64,
    check_interval: Duration,
    tls: Option<Tls>,
}

impl Acme {
    /// Production Let's Encrypt directory.
    pub fn lets_encrypt(domains: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::new(domains, false)
    }

    /// Staging Let's Encrypt directory (rate-limit friendly).
    pub fn lets_encrypt_staging(domains: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::new(domains, true)
    }

    fn new(domains: impl IntoIterator<Item = impl Into<String>>, staging: bool) -> Self {
        Self {
            domains: domains.into_iter().map(Into::into).collect(),
            email: None,
            dir: PathBuf::from("data/acme"),
            staging,
            http_port: 80,
            https_port: 443,
            redirect_https: true,
            renew_days: 30,
            check_interval: Duration::from_secs(12 * 3600),
            tls: None,
        }
    }

    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Directory for `account.json`, `cert.pem`, `key.pem`, `meta.json`.
    pub fn dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.dir = path.into();
        self
    }

    pub fn staging(mut self, staging: bool) -> Self {
        self.staging = staging;
        self
    }

    pub fn http_port(mut self, port: u16) -> Self {
        self.http_port = port;
        self
    }

    /// Port used in `Location` when redirecting non-challenge HTTP traffic.
    pub fn https_port(mut self, port: u16) -> Self {
        self.https_port = port;
        self
    }

    pub fn redirect_https(mut self, on: bool) -> Self {
        self.redirect_https = on;
        self
    }

    /// Renew when remaining lifetime ≤ this many days (default 30).
    pub fn renew_days(mut self, days: u64) -> Self {
        self.renew_days = days.max(1);
        self
    }

    pub fn check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Attach the [`Tls`] returned by [`Self::tls`] (required before `install`).
    pub fn with_tls(mut self, tls: Tls) -> Self {
        self.tls = Some(tls);
        self
    }

    /// Load existing cert from [`Self::dir`] or write a temporary self-signed placeholder.
    pub fn tls(&self) -> Result<Tls> {
        if self.domains.is_empty() {
            return Err(sova_core::Error::Internal(
                "acme: at least one domain is required".into(),
            ));
        }
        let storage = AcmeStorage::new(&self.dir);
        storage.ensure_dir()?;

        if storage.has_cert() {
            return Tls::from_pem(storage.cert_path(), storage.key_path());
        }

        // Placeholder so HTTPS can bind before the first LE issue completes.
        let sans: Vec<&str> = self.domains.iter().map(|s| s.as_str()).collect();
        let cert = rcgen::generate_simple_self_signed(
            sans.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
        )
        .map_err(|e| sova_core::Error::Internal(format!("acme placeholder cert: {e}")))?;
        let cert_pem = cert.cert.pem();
        let key_pem = cert.key_pair.serialize_pem();
        storage.write_pem(&cert_pem, &key_pem)?;
        Tls::from_pem(storage.cert_path(), storage.key_path())
    }
}

impl Plugin for Acme {
    fn id(&self) -> &'static str {
        "acme"
    }

    fn meta(&self) -> PluginMeta {
        PluginMeta::new("ACME")
            .description("Let's Encrypt HTTP-01 certificates with TLS hot-reload")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        let tls = match self.tls {
            Some(t) => t,
            None => {
                tracing::error!("acme: call .with_tls(acme.tls()?) before install");
                return;
            }
        };
        if self.domains.is_empty() {
            tracing::error!("acme: no domains configured");
            return;
        }

        let storage = AcmeStorage::new(&self.dir);
        let _ = storage.ensure_dir();
        let meta = storage.load_meta();
        let placeholder = meta.is_none()
            || meta
                .as_ref()
                .map(|m| m.domains != self.domains || m.staging != self.staging)
                .unwrap_or(true);

        let handle = AcmeHandle {
            status: Arc::new(Mutex::new(AcmeStatus::from_meta(
                &self.domains,
                self.staging,
                meta.as_ref(),
                placeholder || !storage.has_cert(),
            ))),
            force: Arc::new(Notify::new()),
        };

        let events = Some(app.events().clone());
        app.state(handle.clone());

        let handle_cli = handle.clone();
        app.register_cli("acme", move |_state, args| {
            let handle = handle_cli.clone();
            async move {
                match args.first().map(|s| s.as_str()) {
                    Some("status") | None => {
                        let st = handle.status();
                        println!(
                            "domains={:?} staging={} placeholder={} not_after={:?} last_error={:?}",
                            st.domains,
                            st.staging,
                            st.using_placeholder,
                            st.not_after_unix,
                            st.last_error
                        );
                        Ok(())
                    }
                    Some("renew") => {
                        handle.force_renew();
                        println!("acme: renew requested");
                        Ok(())
                    }
                    Some(other) => {
                        eprintln!("usage: acme [status|renew] (got {other})");
                        Err(sova_core::Error::Internal("bad acme args".into()))
                    }
                }
            }
        });

        app.service(AcmeService {
            domains: self.domains,
            email: self.email,
            staging: self.staging,
            storage,
            challenges: ChallengeMap::new(),
            tls,
            handle,
            events,
            http_port: self.http_port,
            https_port: self.https_port,
            redirect_https: self.redirect_https,
            renew_days: self.renew_days,
            check_interval: self.check_interval,
        });

        tracing::info!("acme: installed (HTTP-01 + renewer)");
    }
}

// re-export for docs / tests
pub use storage::CertMeta;

#[cfg(test)]
mod tests_unit;
