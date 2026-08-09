//! Domain events for ACME certificate lifecycle.

use sova_core::Event;

/// Fired after the first successful certificate issuance (or replace of placeholder).
#[derive(Debug, Clone)]
pub struct CertificateIssued {
    pub domains: Vec<String>,
    pub not_after_unix: u64,
}

impl Event for CertificateIssued {
    fn name(&self) -> &'static str {
        "acme.certificate_issued"
    }
}

/// Fired after a successful renewal of an existing certificate.
#[derive(Debug, Clone)]
pub struct CertificateRenewed {
    pub domains: Vec<String>,
    pub not_after_unix: u64,
}

impl Event for CertificateRenewed {
    fn name(&self) -> &'static str {
        "acme.certificate_renewed"
    }
}

/// Fired when an ACME issue/renew attempt fails.
#[derive(Debug, Clone)]
pub struct AcmeFailed {
    pub domains: Vec<String>,
    pub error: String,
}

impl Event for AcmeFailed {
    fn name(&self) -> &'static str {
        "acme.failed"
    }
}
