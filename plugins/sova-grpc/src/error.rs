//! gRPC-style errors (Connect-JSON unary).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrpcError {
    #[error("grpc transport: {0}")]
    Transport(String),
    #[error("grpc http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("grpc decode: {0}")]
    Decode(String),
    #[error("grpc method not found: {0}")]
    NotFound(String),
    #[error("grpc handler: {0}")]
    Handler(String),
    #[error("grpc plugin not installed")]
    NotInstalled,
}

impl From<GrpcError> for sova_core::Error {
    fn from(value: GrpcError) -> Self {
        sova_core::Error::Internal(value.to_string())
    }
}
