//! Shared ACME runtime handle (status + force renew).

use crate::storage::CertMeta;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

#[derive(Debug, Clone)]
pub struct AcmeStatus {
    pub domains: Vec<String>,
    pub staging: bool,
    pub not_after_unix: Option<u64>,
    pub last_error: Option<String>,
    pub last_success_unix: Option<u64>,
    pub using_placeholder: bool,
}

impl AcmeStatus {
    pub fn from_meta(domains: &[String], staging: bool, meta: Option<&CertMeta>, placeholder: bool) -> Self {
        Self {
            domains: domains.to_vec(),
            staging,
            not_after_unix: meta.map(|m| m.not_after_unix),
            last_error: None,
            last_success_unix: None,
            using_placeholder: placeholder,
        }
    }
}

#[derive(Clone)]
pub struct AcmeHandle {
    pub(crate) status: Arc<Mutex<AcmeStatus>>,
    pub(crate) force: Arc<Notify>,
}

impl AcmeHandle {
    pub fn status(&self) -> AcmeStatus {
        self.status.lock().expect("acme status").clone()
    }

    /// Wake the renewer to attempt issue/renew immediately.
    pub fn force_renew(&self) {
        self.force.notify_one();
    }
}
