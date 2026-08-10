//! Transport trait for unary Connect-JSON calls.

use crate::error::GrpcError;
use bytes::Bytes;
use sova_core::extend::BoxFuture;

pub trait GrpcTransport: Send + Sync {
    fn call(&self, base: &str, method: &str, body: Bytes) -> BoxFuture<Result<Bytes, GrpcError>>;
}
