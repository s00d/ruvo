//! DevTools tracing (`target: "sova.rabbit"`).

use crate::error::RabbitError;

pub(crate) fn emit_publish(
    exchange: &str,
    routing_key: &str,
    bytes: u64,
    duration_ms: f64,
    result: &Result<(), RabbitError>,
) {
    let (ok, error) = match result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };
    tracing::debug!(
        target: "sova.rabbit",
        op = "publish",
        exchange = exchange,
        routing_key = routing_key,
        bytes = bytes,
        duration_ms = duration_ms,
        ok = ok,
        error = error.as_deref(),
    );
}

pub(crate) fn emit_consume(
    queue: &str,
    bytes: Option<u64>,
    duration_ms: f64,
    result: &Result<(), RabbitError>,
    empty: bool,
) {
    let (ok, error) = match result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };
    tracing::debug!(
        target: "sova.rabbit",
        op = "consume",
        queue = queue,
        bytes = bytes,
        duration_ms = duration_ms,
        ok = ok,
        empty = empty,
        error = error.as_deref(),
    );
}
