//! Transport seam: real reqwest vs fake.

use crate::error::HttpError;
use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode};
use ruvo_core::extend::BoxFuture;
use std::time::Duration;

/// Outbound request after builder normalization.
#[derive(Debug, Clone)]
pub struct OutRequest {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
    pub body: Option<Bytes>,
    pub timeout: Option<Duration>,
}

/// Thin response wrapper (not a parallel HTTP stack).
#[derive(Debug, Clone)]
pub struct OutResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl OutResponse {
    pub fn new(status: StatusCode, headers: HeaderMap, body: Bytes) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn status_u16(&self) -> u16 {
        self.status.as_u16()
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn bytes(&self) -> &Bytes {
        &self.body
    }

    pub fn text(&self) -> Result<String, HttpError> {
        String::from_utf8(self.body.to_vec()).map_err(|e| HttpError::Other(e.to_string()))
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, HttpError> {
        serde_json::from_slice(&self.body).map_err(|e| HttpError::Other(e.to_string()))
    }

    /// Treat 4xx/5xx as [`HttpError::Status`] (5xx → 502 via IntoResponse).
    pub fn error_for_status(self) -> Result<Self, HttpError> {
        let code = self.status.as_u16();
        if code >= 400 {
            Err(HttpError::Status(code))
        } else {
            Ok(self)
        }
    }
}

pub trait Transport: Send + Sync {
    fn send(&self, req: OutRequest) -> BoxFuture<Result<OutResponse, HttpError>>;
}
