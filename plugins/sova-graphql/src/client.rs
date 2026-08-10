//! Outbound GraphQL client (`req.graphql().query / mutation`).

use crate::error::GraphqlError;
use crate::fake::FakeGraphql;
use bytes::Bytes;
use serde::Serialize;
use serde_json::{json, Value};
use sova_core::extend::BoxFuture;
use std::sync::Arc;

/// Full GraphQL HTTP response JSON (`data` + optional `errors`).
#[derive(Debug, Clone)]
pub struct GraphqlResponse {
    pub raw: Value,
}

impl GraphqlResponse {
    pub fn data(&self) -> Option<&Value> {
        self.raw.get("data")
    }

    pub fn errors(&self) -> Option<&Value> {
        self.raw.get("errors")
    }

    /// Prefer `data`; Err if GraphQL `errors` present or data missing.
    pub fn into_data(self) -> Result<Value, GraphqlError> {
        if let Some(errs) = self.raw.get("errors") {
            if !errs.is_null() && errs.as_array().map(|a| !a.is_empty()).unwrap_or(true) {
                return Err(GraphqlError::Graphql(errs.to_string()));
            }
        }
        self.raw
            .get("data")
            .cloned()
            .ok_or_else(|| GraphqlError::Decode("missing data field".into()))
    }
}

pub(crate) trait GraphqlTransport: Send + Sync {
    fn post(&self, url: &str, body: Bytes) -> BoxFuture<Result<Bytes, GraphqlError>>;
}

struct HttpTransport {
    client: reqwest::Client,
}

impl GraphqlTransport for HttpTransport {
    fn post(&self, url: &str, body: Bytes) -> BoxFuture<Result<Bytes, GraphqlError>> {
        let client = self.client.clone();
        let url = url.to_string();
        Box::pin(async move {
            let res = client
                .post(url)
                .header(http::header::CONTENT_TYPE, "application/json")
                .body(body)
                .send()
                .await
                .map_err(|e| GraphqlError::Transport(e.to_string()))?;
            let status = res.status().as_u16();
            let bytes = res
                .bytes()
                .await
                .map_err(|e| GraphqlError::Transport(e.to_string()))?;
            if !(200..300).contains(&status) {
                return Err(GraphqlError::Http {
                    status,
                    body: String::from_utf8_lossy(&bytes).into_owned(),
                });
            }
            Ok(bytes)
        })
    }
}

/// Shared client stored in app state.
#[derive(Clone)]
pub struct GraphQlClient {
    endpoint: String,
    transport: Arc<dyn GraphqlTransport>,
    fake: Option<FakeGraphql>,
}

impl GraphQlClient {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn fake(&self) -> Option<&FakeGraphql> {
        self.fake.as_ref()
    }

    pub fn query(&self, query: impl Into<String>) -> PendingGraphql {
        PendingGraphql {
            client: self.clone(),
            query: query.into(),
            operation_name: None,
            variables: None,
        }
    }

    pub fn mutation(&self, query: impl Into<String>) -> PendingGraphql {
        self.query(query)
    }

    pub(crate) fn http(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            transport: Arc::new(HttpTransport {
                client: reqwest::Client::new(),
            }),
            fake: None,
        }
    }

    pub(crate) fn with_fake(endpoint: impl Into<String>, fake: FakeGraphql) -> Self {
        Self {
            endpoint: endpoint.into(),
            transport: Arc::new(fake.clone()),
            fake: Some(fake),
        }
    }

    pub(crate) async fn execute_raw(
        &self,
        query: &str,
        operation_name: Option<&str>,
        variables: Option<&Value>,
    ) -> Result<GraphqlResponse, GraphqlError> {
        let body = json!({
            "query": query,
            "operationName": operation_name,
            "variables": variables.unwrap_or(&Value::Null),
        });
        let bytes = Bytes::from(
            serde_json::to_vec(&body).map_err(|e| GraphqlError::Decode(e.to_string()))?,
        );
        let resp = self.transport.post(&self.endpoint, bytes).await?;
        let raw: Value = serde_json::from_slice(&resp)
            .map_err(|e| GraphqlError::Decode(e.to_string()))?;
        Ok(GraphqlResponse { raw })
    }
}

/// Fluent GraphQL request builder.
pub struct PendingGraphql {
    client: GraphQlClient,
    query: String,
    operation_name: Option<String>,
    variables: Option<Value>,
}

impl PendingGraphql {
    pub fn operation_name(mut self, name: impl Into<String>) -> Self {
        self.operation_name = Some(name.into());
        self
    }

    pub fn variables<V: Serialize>(mut self, vars: V) -> Self {
        self.variables = Some(serde_json::to_value(vars).unwrap_or(Value::Null));
        self
    }

    /// Full response JSON.
    pub async fn send(self) -> Result<GraphqlResponse, GraphqlError> {
        self.client
            .execute_raw(
                &self.query,
                self.operation_name.as_deref(),
                self.variables.as_ref(),
            )
            .await
    }

    /// `data` field only (errors → [`GraphqlError::Graphql`]).
    pub async fn data(self) -> Result<Value, GraphqlError> {
        self.send().await?.into_data()
    }

    /// Alias of [`Self::data`].
    pub async fn await_data(self) -> Result<Value, GraphqlError> {
        self.data().await
    }
}
