//! Fake GraphQL transport for outbound client tests.

use crate::client::GraphqlTransport;
use crate::error::GraphqlError;
use bytes::Bytes;
use serde_json::{json, Value};
use sova_core::extend::BoxFuture;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct GraphqlCall {
    pub query: String,
    pub operation_name: Option<String>,
    pub variables: Option<Value>,
    pub endpoint: String,
}

#[derive(Default)]
struct Inner {
    /// (query substring, data JSON)
    stubs: Vec<(String, Value)>,
    calls: Vec<GraphqlCall>,
}

/// Stub outbound GraphQL by query substring; records every call.
#[derive(Clone, Default)]
pub struct FakeGraphql {
    inner: Arc<Mutex<Inner>>,
}

impl FakeGraphql {
    pub fn new() -> Self {
        Self::default()
    }

    /// When the query contains `needle`, respond with `{ "data": body }`.
    pub fn stub(self, needle: impl Into<String>, data: Value) -> Self {
        self.inner
            .lock()
            .unwrap()
            .stubs
            .push((needle.into(), data));
        self
    }

    pub fn calls(&self) -> Vec<GraphqlCall> {
        self.inner.lock().unwrap().calls.clone()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().calls.clear();
    }

    pub fn assert_called(&self) {
        assert!(
            !self.inner.lock().unwrap().calls.is_empty(),
            "FakeGraphql: expected at least one GraphQL call"
        );
    }

    pub fn assert_called_with(&self, needle: &str) {
        let calls = self.calls();
        assert!(
            calls.iter().any(|c| c.query.contains(needle)),
            "FakeGraphql: no call matched `{needle}`; calls={calls:?}"
        );
    }
}

impl GraphqlTransport for FakeGraphql {
    fn post(&self, url: &str, body: Bytes) -> BoxFuture<Result<Bytes, GraphqlError>> {
        let this = self.clone();
        let url = url.to_string();
        Box::pin(async move {
            let payload: Value = serde_json::from_slice(&body)
                .map_err(|e| GraphqlError::Decode(e.to_string()))?;
            let query = payload
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let operation_name = payload
                .get("operationName")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let variables = payload.get("variables").cloned();

            let mut g = this.inner.lock().unwrap();
            g.calls.push(GraphqlCall {
                query: query.clone(),
                operation_name,
                variables,
                endpoint: url,
            });

            for (needle, data) in &g.stubs {
                if query.contains(needle) {
                    let resp = json!({ "data": data });
                    return serde_json::to_vec(&resp)
                        .map(Bytes::from)
                        .map_err(|e| GraphqlError::Decode(e.to_string()));
                }
            }

            Err(GraphqlError::Graphql(format!(
                "FakeGraphql: no stub matched query ({query})"
            )))
        })
    }
}
