//! Background renewer + HTTP-01 listener.

use crate::events::{AcmeFailed, CertificateIssued, CertificateRenewed};
use crate::handle::AcmeHandle;
use crate::http01::{run_http01_listener, ChallengeMap};
use crate::issue::obtain_certificate;
use crate::storage::{needs_renew, not_after_from_pem, now_unix, AcmeStorage, CertMeta};
use sova_core::extend::{BoxFuture, StateMap};
use sova_core::{BackgroundService, EventBus, Shutdown, Tls};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

pub struct AcmeService {
    pub domains: Vec<String>,
    pub email: Option<String>,
    pub staging: bool,
    pub storage: AcmeStorage,
    pub challenges: ChallengeMap,
    pub tls: Tls,
    pub handle: AcmeHandle,
    pub events: Option<EventBus>,
    pub http_port: u16,
    pub https_port: u16,
    pub redirect_https: bool,
    pub renew_days: u64,
    pub check_interval: Duration,
}

impl BackgroundService for AcmeService {
    fn name(&self) -> &str {
        "acme"
    }

    fn run(self: Box<Self>, _state: Arc<StateMap>, shutdown: Shutdown) -> BoxFuture<()> {
        Box::pin(async move {
            let (stop_tx, stop_rx) = watch::channel(false);
            let http_task = {
                let challenges = self.challenges.clone();
                let port = self.http_port;
                let redirect = self.redirect_https;
                let https_port = self.https_port;
                let rx = stop_rx.clone();
                tokio::spawn(async move {
                    run_http01_listener(port, challenges, redirect, https_port, rx).await;
                })
            };

            // Initial issue if placeholder / missing / expired.
            maybe_issue(&self, true).await;

            let mut ticker = tokio::time::interval(self.check_interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // Skip immediate tick — we just ran maybe_issue.
            ticker.tick().await;

            loop {
                tokio::select! {
                    _ = shutdown_wait(shutdown.clone()) => break,
                    _ = self.handle.force.notified() => {
                        maybe_issue(&self, true).await;
                    }
                    _ = ticker.tick() => {
                        maybe_issue(&self, false).await;
                    }
                }
            }

            let _ = stop_tx.send(true);
            let _ = http_task.await;
        })
    }
}

async fn shutdown_wait(mut shutdown: Shutdown) {
    shutdown.recv().await;
}

async fn maybe_issue(svc: &AcmeService, force: bool) {
    let meta = svc.storage.load_meta();
    let placeholder = {
        svc.handle
            .status
            .lock()
            .expect("acme status")
            .using_placeholder
    };

    let should = force
        || placeholder
        || meta
            .as_ref()
            .map(|m| needs_renew(m, svc.renew_days))
            .unwrap_or(true);

    if !should {
        return;
    }

    let was_placeholder = placeholder || !svc.storage.has_cert();
    tracing::info!(
        domains = ?svc.domains,
        staging = svc.staging,
        force,
        "acme: obtaining certificate"
    );

    match obtain_certificate(
        &svc.domains,
        svc.email.as_deref(),
        svc.staging,
        &svc.storage,
        &svc.challenges,
    )
    .await
    {
        Ok(issued) => {
            if let Err(e) = svc.tls.reload_pem(&issued.cert_pem, &issued.key_pem) {
                fail(svc, format!("reload_pem: {e}"));
                return;
            }
            let not_after = not_after_from_pem(&issued.cert_pem).unwrap_or(0);
            let meta = CertMeta {
                domains: svc.domains.clone(),
                not_after_unix: not_after,
                staging: svc.staging,
            };
            if let Err(e) = svc.storage.save_meta(&meta) {
                tracing::warn!(error = %e, "acme: meta save failed");
            }
            {
                let mut st = svc.handle.status.lock().expect("acme status");
                st.not_after_unix = Some(not_after);
                st.last_error = None;
                st.last_success_unix = Some(now_unix());
                st.using_placeholder = false;
            }
            if let Some(bus) = &svc.events {
                if was_placeholder {
                    bus.dispatch(CertificateIssued {
                        domains: svc.domains.clone(),
                        not_after_unix: not_after,
                    });
                } else {
                    bus.dispatch(CertificateRenewed {
                        domains: svc.domains.clone(),
                        not_after_unix: not_after,
                    });
                }
            }
            tracing::info!(not_after_unix = not_after, "acme: certificate installed");
        }
        Err(e) => fail(svc, e.to_string()),
    }
}

fn fail(svc: &AcmeService, error: String) {
    tracing::error!(error = %error, "acme: certificate obtain failed");
    {
        let mut st = svc.handle.status.lock().expect("acme status");
        st.last_error = Some(error.clone());
    }
    if let Some(bus) = &svc.events {
        bus.dispatch(AcmeFailed {
            domains: svc.domains.clone(),
            error,
        });
    }
}
