//! DevTools-friendly query tracing at INFO (sqlx defaults to DEBUG).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_sql_trace(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn log_query(sql: &str, started: Instant, ok: bool) {
    if !enabled() {
        return;
    }
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    if let Some(rid) = sova_core::current_request_id() {
        if ok {
            tracing::info!(
                target: "sova.db",
                request_id = %rid,
                elapsed = elapsed,
                "{sql}"
            );
        } else {
            tracing::warn!(
                target: "sova.db",
                request_id = %rid,
                elapsed = elapsed,
                "{sql}"
            );
        }
    } else if ok {
        tracing::info!(
            target: "sova.db",
            elapsed = elapsed,
            "{sql}"
        );
    } else {
        tracing::warn!(
            target: "sova.db",
            elapsed = elapsed,
            "{sql}"
        );
    }
}
