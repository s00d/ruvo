//! In-memory fake transport for tests.

use crate::error::HttpError;
use crate::transport::{OutRequest, OutResponse, Transport};
use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode};
use sova_core::extend::BoxFuture;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub enum StubBody {
    Empty,
    Bytes(Bytes),
    Json(serde_json::Value),
}

#[derive(Debug, Clone)]
pub enum StubOutcome {
    Response {
        status: StatusCode,
        headers: HeaderMap,
        body: StubBody,
    },
    Fail(String),
}

#[derive(Debug, Clone)]
struct Stub {
    method: Option<Method>,
    url_pattern: String,
    outcome: StubOutcome,
}

#[derive(Debug, Default)]
struct FakeInner {
    stubs: Vec<Stub>,
    sent: Vec<OutRequest>,
}

/// Laravel-style `Http::fake()` registry.
#[derive(Clone, Default)]
pub struct FakeTransport {
    inner: std::sync::Arc<Mutex<FakeInner>>,
}

impl FakeTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(self, url: impl Into<String>, body: impl Into<StubBody>) -> Self {
        self.stub(Some(Method::GET), url, StubOutcome::Response {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: body.into(),
        })
    }

    pub fn post(self, url: impl Into<String>, status: u16) -> Self {
        self.stub(
            Some(Method::POST),
            url,
            StubOutcome::Response {
                status: StatusCode::from_u16(status).unwrap_or(StatusCode::CREATED),
                headers: HeaderMap::new(),
                body: StubBody::Empty,
            },
        )
    }

    pub fn respond(
        self,
        method: Method,
        url: impl Into<String>,
        status: u16,
        body: impl Into<StubBody>,
    ) -> Self {
        self.stub(
            Some(method),
            url,
            StubOutcome::Response {
                status: StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
                headers: HeaderMap::new(),
                body: body.into(),
            },
        )
    }

    pub fn fail(self, url: impl Into<String>, msg: impl Into<String>) -> Self {
        self.stub(None, url, StubOutcome::Fail(msg.into()))
    }

    fn stub(self, method: Option<Method>, url: impl Into<String>, outcome: StubOutcome) -> Self {
        self.inner.lock().unwrap().stubs.push(Stub {
            method,
            url_pattern: url.into(),
            outcome,
        });
        self
    }

    pub fn assert_sent(&self, method: Method, url: &str) {
        let g = self.inner.lock().unwrap();
        assert!(
            g.sent
                .iter()
                .any(|r| r.method == method && url_matches(&r.url, url)),
            "expected {method} {url}, sent: {:?}",
            g.sent
                .iter()
                .map(|r| format!("{} {}", r.method, r.url))
                .collect::<Vec<_>>()
        );
    }

    pub fn assert_sent_count(&self, n: usize) {
        let g = self.inner.lock().unwrap();
        assert_eq!(g.sent.len(), n, "sent {:?}", g.sent.len());
    }

    pub fn assert_not_sent(&self, url: &str) {
        let g = self.inner.lock().unwrap();
        assert!(
            !g.sent.iter().any(|r| url_matches(&r.url, url)),
            "unexpected call to {url}"
        );
    }

    pub fn sent(&self) -> Vec<OutRequest> {
        self.inner.lock().unwrap().sent.clone()
    }
}

impl From<serde_json::Value> for StubBody {
    fn from(v: serde_json::Value) -> Self {
        Self::Json(v)
    }
}

impl From<&'static str> for StubBody {
    fn from(s: &'static str) -> Self {
        Self::Bytes(Bytes::from_static(s.as_bytes()))
    }
}

impl From<String> for StubBody {
    fn from(s: String) -> Self {
        Self::Bytes(Bytes::from(s))
    }
}

fn url_matches(actual: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        actual.starts_with(prefix)
    } else {
        actual == pattern
    }
}

fn stub_to_response(outcome: &StubOutcome) -> Result<OutResponse, HttpError> {
    match outcome {
        StubOutcome::Fail(m) => {
            if m.to_ascii_lowercase().contains("timeout") {
                Err(HttpError::Timeout)
            } else {
                Err(HttpError::Other(m.clone()))
            }
        }
        StubOutcome::Response {
            status,
            headers,
            body,
        } => {
            let bytes = match body {
                StubBody::Empty => Bytes::new(),
                StubBody::Bytes(b) => b.clone(),
                StubBody::Json(v) => Bytes::from(serde_json::to_vec(v).unwrap_or_default()),
            };
            Ok(OutResponse::new(*status, headers.clone(), bytes))
        }
    }
}

impl Transport for FakeTransport {
    fn send(&self, req: OutRequest) -> BoxFuture<Result<OutResponse, HttpError>> {
        let this = self.clone();
        Box::pin(async move {
            let mut g = this.inner.lock().unwrap();
            g.sent.push(req.clone());
            let outcome = g
                .stubs
                .iter()
                .find(|s| {
                    s.method.as_ref().map(|m| *m == req.method).unwrap_or(true)
                        && url_matches(&req.url, &s.url_pattern)
                })
                .map(|s| s.outcome.clone());
            drop(g);
            match outcome {
                Some(o) => stub_to_response(&o),
                None => Err(HttpError::Other(format!(
                    "no fake stub for {} {}",
                    req.method, req.url
                ))),
            }
        })
    }
}
