//! Outbound HTTP errors mapped to gateway-style responses.

use ruvo_core::extend::ErrorResponse;
use ruvo_core::{Error, IntoResponse, Response};
use std::fmt;

/// Failure of an outbound HTTP call (not an upstream 4xx body).
#[derive(Debug)]
pub enum HttpError {
    Timeout,
    Connect(String),
    ResponseTooLarge,
    Ssrf(String),
    CircuitOpen(String),
    Status(u16),
    Other(String),
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(f, "upstream timeout"),
            Self::Connect(m) => write!(f, "upstream connect: {m}"),
            Self::ResponseTooLarge => write!(f, "upstream response too large"),
            Self::Ssrf(m) => write!(f, "ssrf blocked: {m}"),
            Self::CircuitOpen(host) => write!(f, "circuit open for {host}"),
            Self::Status(c) => write!(f, "upstream status {c}"),
            Self::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for HttpError {}

impl HttpError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Timeout => 504,
            Self::Connect(_)
            | Self::ResponseTooLarge
            | Self::Ssrf(_)
            | Self::CircuitOpen(_)
            | Self::Status(_)
            | Self::Other(_) => 502,
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let msg = match status {
            504 => "Gateway Timeout",
            _ => "Bad Gateway",
        };
        Response::text(msg).status(status)
    }
}

impl From<HttpError> for Error {
    fn from(err: HttpError) -> Self {
        Error::Response(Box::new(err.into_response()))
    }
}

impl ErrorResponse for HttpError {}

impl From<reqwest::Error> for HttpError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            return Self::Timeout;
        }
        if err.is_connect() {
            return Self::Connect(err.to_string());
        }
        Self::Other(err.to_string())
    }
}
