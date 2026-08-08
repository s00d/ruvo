//! Outbound HTTP client for Ruvo — request-bound deadline, tracing, fake transport.

mod bound;
mod breaker;
mod client;
mod error;
mod fake;
mod reqwest_transport;
mod retry;
mod ssrf;
mod transport;

pub use bound::{HttpBound, HttpExt, NamedBound};
pub use breaker::{BreakerConfig, CircuitBreaker};
pub use client::{Http, HttpClient, NamedClient, NamedClientConfig, PendingRequest};
pub use error::HttpError;
pub use fake::{FakeTransport, StubBody, StubOutcome};
pub use transport::{OutRequest, OutResponse, Transport};

/// Re-export core correlation id (set by `ruvo_core::request_id()` middleware).
pub use ruvo_core::RequestId;
