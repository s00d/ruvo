//! On-disk ACME account + certificate storage.

use serde::{Deserialize, Serialize};
use sova_core::{Error, Result};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CertMeta {
    pub domains: Vec<String>,
    pub not_after_unix: u64,
    pub staging: bool,
}

#[derive(Clone, Debug)]
pub struct AcmeStorage {
    root: PathBuf,
}

impl AcmeStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn cert_path(&self) -> PathBuf {
        self.root.join("cert.pem")
    }

    pub fn key_path(&self) -> PathBuf {
        self.root.join("key.pem")
    }

    pub fn account_path(&self) -> PathBuf {
        self.root.join("account.json")
    }

    pub fn meta_path(&self) -> PathBuf {
        self.root.join("meta.json")
    }

    pub fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root)
            .map_err(|e| Error::Internal(format!("acme dir {}: {e}", self.root.display())))
    }

    pub fn has_cert(&self) -> bool {
        self.cert_path().is_file() && self.key_path().is_file()
    }

    pub fn load_meta(&self) -> Option<CertMeta> {
        let raw = std::fs::read_to_string(self.meta_path()).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn save_meta(&self, meta: &CertMeta) -> Result<()> {
        self.ensure_dir()?;
        let raw = serde_json::to_string_pretty(meta)
            .map_err(|e| Error::Internal(format!("acme meta serialize: {e}")))?;
        std::fs::write(self.meta_path(), raw)
            .map_err(|e| Error::Internal(format!("acme meta write: {e}")))
    }

    pub fn load_account_json(&self) -> Option<String> {
        std::fs::read_to_string(self.account_path()).ok()
    }

    pub fn save_account_json(&self, json: &str) -> Result<()> {
        self.ensure_dir()?;
        std::fs::write(self.account_path(), json)
            .map_err(|e| Error::Internal(format!("acme account write: {e}")))
    }

    pub fn write_pem(&self, cert_pem: &str, key_pem: &str) -> Result<()> {
        self.ensure_dir()?;
        std::fs::write(self.cert_path(), cert_pem)
            .map_err(|e| Error::Internal(format!("acme cert write: {e}")))?;
        std::fs::write(self.key_path(), key_pem)
            .map_err(|e| Error::Internal(format!("acme key write: {e}")))
    }
}

pub fn not_after_from_pem(cert_pem: &str) -> Option<u64> {
    let (_, pem) = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes()).ok()?;
    let (_, cert) = x509_parser::parse_x509_certificate(pem.contents.as_ref()).ok()?;
    let ts = cert.validity().not_after.timestamp();
    if ts < 0 {
        None
    } else {
        Some(ts as u64)
    }
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn needs_renew(meta: &CertMeta, renew_days: u64) -> bool {
    let now = now_unix();
    let threshold = renew_days.saturating_mul(24 * 3600);
    meta.not_after_unix.saturating_sub(now) <= threshold
}
