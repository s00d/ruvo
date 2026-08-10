//! DevTools tracing (`target: "sova.grpc"`).

use crate::error::GrpcError;

pub(crate) struct Emit<'a> {
    pub direction: &'a str,
    pub method: &'a str,
    pub base: &'a str,
    pub duration_ms: f64,
    pub ok: bool,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub bytes_in: Option<u64>,
    pub bytes_out: Option<u64>,
}

pub(crate) fn emit(e: Emit<'_>) {
    tracing::debug!(
        target: "sova.grpc",
        direction = e.direction,
        method = %e.method,
        base = %e.base,
        duration_ms = e.duration_ms,
        ok = e.ok,
        status = e.status,
        error = e.error.as_deref(),
        bytes_in = e.bytes_in,
        bytes_out = e.bytes_out,
    );
}

pub(crate) fn emit_client(
    method: &str,
    base: &str,
    duration_ms: f64,
    result: &Result<bytes::Bytes, GrpcError>,
    bytes_in: u64,
) {
    match result {
        Ok(out) => emit(Emit {
            direction: "client",
            method,
            base,
            duration_ms,
            ok: true,
            status: Some(200),
            error: None,
            bytes_in: Some(bytes_in),
            bytes_out: Some(out.len() as u64),
        }),
        Err(e) => {
            let (status, msg) = error_meta(e);
            emit(Emit {
                direction: "client",
                method,
                base,
                duration_ms,
                ok: false,
                status,
                error: Some(msg),
                bytes_in: Some(bytes_in),
                bytes_out: None,
            });
        }
    }
}

pub(crate) fn emit_server(
    method: &str,
    duration_ms: f64,
    result: &Result<bytes::Bytes, GrpcError>,
    bytes_in: u64,
) {
    match result {
        Ok(out) => emit(Emit {
            direction: "server",
            method,
            base: "in-process",
            duration_ms,
            ok: true,
            status: Some(200),
            error: None,
            bytes_in: Some(bytes_in),
            bytes_out: Some(out.len() as u64),
        }),
        Err(e) => {
            let (status, msg) = error_meta(e);
            emit(Emit {
                direction: "server",
                method,
                base: "in-process",
                duration_ms,
                ok: false,
                status,
                error: Some(msg),
                bytes_in: Some(bytes_in),
                bytes_out: None,
            });
        }
    }
}

fn error_meta(err: &GrpcError) -> (Option<u16>, String) {
    match err {
        GrpcError::Http { status, body } => (Some(*status), body.clone()),
        GrpcError::Rpc { code, message } => (None, format!("{code}: {message}")),
        other => (None, other.to_string()),
    }
}
