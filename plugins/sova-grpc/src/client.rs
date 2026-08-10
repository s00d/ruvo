//! Outbound Connect-JSON client.

use crate::error::GrpcError;
use crate::fake::FakeGrpc;
use crate::transport::GrpcTransport;
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use sova_core::extend::BoxFuture;
use std::sync::Arc;

struct HttpTransport {
    client: reqwest::Client,
}

impl GrpcTransport for HttpTransport {
    fn call(&self, base: &str, method: &str, body: Bytes) -> BoxFuture<Result<Bytes, GrpcError>> {
        let client = self.client.clone();
        let url = format!("{}/{}", base.trim_end_matches('/'), method.trim_start_matches('/'));
        Box::pin(async move {
            let res = client
                .post(url)
                .header(http::header::CONTENT_TYPE, "application/json")
                .header("connect-protocol-version", "1")
                .body(body)
                .send()
                .await
                .map_err(|e| GrpcError::Transport(e.to_string()))?;
            let status = res.status().as_u16();
            let bytes = res
                .bytes()
                .await
                .map_err(|e| GrpcError::Transport(e.to_string()))?;
            if !(200..300).contains(&status) {
                return Err(GrpcError::Http {
                    status,
                    body: String::from_utf8_lossy(&bytes).into_owned(),
                });
            }
            Ok(bytes)
        })
    }
}

/// Shared client in app state.
#[derive(Clone)]
pub struct GrpcClient {
    base: String,
    transport: Arc<dyn GrpcTransport>,
    fake: Option<FakeGrpc>,
}

impl GrpcClient {
    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn fake(&self) -> Option<&FakeGrpc> {
        self.fake.as_ref()
    }

    pub(crate) fn http(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            transport: Arc::new(HttpTransport {
                client: reqwest::Client::new(),
            }),
            fake: None,
        }
    }

    pub(crate) fn with_fake(base: impl Into<String>, fake: FakeGrpc) -> Self {
        Self {
            base: base.into(),
            transport: Arc::new(fake.clone()),
            fake: Some(fake),
        }
    }

    pub async fn call<Req, Res>(&self, method: &str, req: &Req) -> Result<Res, GrpcError>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        let body = Bytes::from(
            serde_json::to_vec(req).map_err(|e| GrpcError::Decode(e.to_string()))?,
        );
        let bytes = self.transport.call(&self.base, method, body).await?;
        serde_json::from_slice(&bytes).map_err(|e| GrpcError::Decode(e.to_string()))
    }

    pub async fn call_raw(&self, method: &str, body: Bytes) -> Result<Bytes, GrpcError> {
        self.transport.call(&self.base, method, body).await
    }
}
