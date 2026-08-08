//! Prometheus recorder install for Ruvo.

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::{Mutex, OnceLock};

static HANDLE: OnceLock<Mutex<Option<PrometheusHandle>>> = OnceLock::new();

/// Install (or reuse) the global Prometheus recorder and return a scrape handle.
pub fn install_recorder() -> PrometheusHandle {
    let slot = HANDLE.get_or_init(|| Mutex::new(None));
    let mut g = slot.lock().unwrap();
    if let Some(h) = g.as_ref() {
        return h.clone();
    }
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("prometheus recorder");
    *g = Some(handle.clone());
    handle
}

pub use metrics_exporter_prometheus::PrometheusHandle as Handle;
