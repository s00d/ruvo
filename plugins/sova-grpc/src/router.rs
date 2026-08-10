//! In-process unary method registry.

use crate::error::GrpcError;
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use sova_core::extend::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;

type Handler = Arc<dyn Fn(Bytes) -> BoxFuture<Result<Bytes, GrpcError>> + Send + Sync>;

#[derive(Clone, Default)]
pub struct MethodRouter {
    handlers: Arc<std::sync::RwLock<HashMap<String, Handler>>>,
}

impl MethodRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn unary<Req, Res, F, Fut>(&self, method: impl Into<String>, f: F) -> &Self
    where
        Req: DeserializeOwned + Send + 'static,
        Res: Serialize + Send + 'static,
        F: Fn(Req) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Res, GrpcError>> + Send + 'static,
    {
        let method = method.into();
        let f = Arc::new(f);
        let handler: Handler = Arc::new(move |body| {
            let f = Arc::clone(&f);
            Box::pin(async move {
                let req: Req =
                    serde_json::from_slice(&body).map_err(|e| GrpcError::Decode(e.to_string()))?;
                let res = f(req).await?;
                let bytes = serde_json::to_vec(&res).map_err(|e| GrpcError::Decode(e.to_string()))?;
                Ok(Bytes::from(bytes))
            })
        });
        self.handlers.write().unwrap().insert(method, handler);
        self
    }

    pub async fn invoke_raw(&self, method: &str, body: Bytes) -> Result<Bytes, GrpcError> {
        let handler = self
            .handlers
            .read()
            .unwrap()
            .get(method)
            .cloned()
            .ok_or_else(|| GrpcError::NotFound(method.to_string()))?;
        handler(body).await
    }

    pub async fn invoke<Req, Res>(&self, method: &str, req: &Req) -> Result<Res, GrpcError>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        let body = Bytes::from(
            serde_json::to_vec(req).map_err(|e| GrpcError::Decode(e.to_string()))?,
        );
        let out = self.invoke_raw(method, body).await?;
        serde_json::from_slice(&out).map_err(|e| GrpcError::Decode(e.to_string()))
    }

    pub fn methods(&self) -> Vec<String> {
        self.handlers.read().unwrap().keys().cloned().collect()
    }
}
