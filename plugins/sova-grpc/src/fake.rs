//! Fake unary transport for tests.

use crate::error::GrpcError;
use crate::transport::GrpcTransport;
use bytes::Bytes;
use serde_json::Value;
use sova_core::extend::BoxFuture;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct GrpcCall {
    pub method: String,
    pub body: Bytes,
    pub base: String,
}

#[derive(Default)]
struct Inner {
    stubs: Vec<(String, Bytes)>,
    calls: Vec<GrpcCall>,
}

#[derive(Clone, Default)]
pub struct FakeGrpc {
    inner: Arc<Mutex<Inner>>,
}

impl FakeGrpc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stub(self, method: impl Into<String>, body: impl Into<Bytes>) -> Self {
        self.inner
            .lock()
            .unwrap()
            .stubs
            .push((method.into(), body.into()));
        self
    }

    pub fn stub_json(self, method: impl Into<String>, value: Value) -> Self {
        let bytes = Bytes::from(serde_json::to_vec(&value).expect("json"));
        self.stub(method, bytes)
    }

    pub fn calls(&self) -> Vec<GrpcCall> {
        self.inner.lock().unwrap().calls.clone()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().calls.clear();
    }

    pub fn assert_called(&self) {
        assert!(
            !self.inner.lock().unwrap().calls.is_empty(),
            "FakeGrpc: expected at least one call"
        );
    }

    pub fn assert_called_method(&self, method: &str) {
        let calls = self.calls();
        assert!(
            calls.iter().any(|c| c.method == method),
            "FakeGrpc: no call to `{method}`; calls={:?}",
            calls.iter().map(|c| &c.method).collect::<Vec<_>>()
        );
    }
}

impl GrpcTransport for FakeGrpc {
    fn call(&self, base: &str, method: &str, body: Bytes) -> BoxFuture<Result<Bytes, GrpcError>> {
        let this = self.clone();
        let base = base.to_string();
        let method = method.to_string();
        Box::pin(async move {
            let mut g = this.inner.lock().unwrap();
            g.calls.push(GrpcCall {
                method: method.clone(),
                body,
                base,
            });
            for (m, resp) in &g.stubs {
                if m == &method {
                    return Ok(resp.clone());
                }
            }
            Err(GrpcError::NotFound(method))
        })
    }
}
