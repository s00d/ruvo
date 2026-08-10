//! Connect-JSON error envelope (shared client + server).

use crate::error::GrpcError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectError {
    pub code: String,
    pub message: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub details: Option<Value>,
}

pub fn parse_connect_error(body: &str) -> Option<ConnectError> {
    serde_json::from_str(body).ok()
}

pub fn connect_error_json(code: impl Into<String>, message: impl Into<String>) -> Value {
    serde_json::json!({
        "code": code.into(),
        "message": message.into(),
    })
}

pub fn grpc_error_to_connect(err: &GrpcError) -> (u16, String, String) {
    match err {
        GrpcError::NotFound(m) => (404, "not_found".into(), m.clone()),
        GrpcError::Decode(m) => (400, "invalid_argument".into(), m.clone()),
        GrpcError::Handler(m) => (500, "internal".into(), m.clone()),
        GrpcError::Rpc { code, message } => (400, code.clone(), message.clone()),
        GrpcError::Transport(m) => (502, "unavailable".into(), m.clone()),
        GrpcError::Http { status, body } => {
            if let Some(parsed) = parse_connect_error(body) {
                (*status, parsed.code, parsed.message)
            } else {
                (*status, "unknown".into(), body.clone())
            }
        }
        GrpcError::NotInstalled => (503, "unavailable".into(), err.to_string()),
    }
}

pub fn status_for_rpc_code(code: &str) -> u16 {
    match code {
        "invalid_argument" | "failed_precondition" | "out_of_range" => 400,
        "not_found" => 404,
        "already_exists" | "aborted" => 409,
        "permission_denied" => 403,
        "unauthenticated" => 401,
        "resource_exhausted" => 429,
        "unimplemented" => 501,
        "unavailable" => 503,
        _ => 500,
    }
}
