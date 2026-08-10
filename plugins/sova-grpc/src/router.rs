//! In-process unary method registry.

use crate::error::GrpcError;
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use sova_core::extend::BoxFuture;
use sova_core::Request;
use std::collections::HashMap;
use std::sync::Arc;

type BodyHandler = Arc<dyn Fn(Bytes) -> BoxFuture<Result<Bytes, GrpcError>> + Send + Sync>;
type CtxHandler = Arc<dyn Fn(Request, Bytes) -> BoxFuture<Result<Bytes, GrpcError>> + Send + Sync>;

#[derive(Clone)]
enum Handler {
    Body(BodyHandler),
    Ctx(CtxHandler),
}

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
        let handler: BodyHandler = Arc::new(move |body| {
            let f = Arc::clone(&f);
            Box::pin(async move {
                let req: Req =
                    serde_json::from_slice(&body).map_err(|e| GrpcError::Decode(e.to_string()))?;
                let res = f(req).await?;
                let bytes =
                    serde_json::to_vec(&res).map_err(|e| GrpcError::Decode(e.to_string()))?;
                Ok(Bytes::from(bytes))
            })
        });
        self.handlers
            .write()
            .unwrap()
            .insert(method, Handler::Body(handler));
        self
    }

    /// Unary handler with access to the incoming HTTP [`Request`] (auth, state, headers).
    pub fn unary_with_request<Req, Res, F, Fut>(&self, method: impl Into<String>, f: F) -> &Self
    where
        Req: DeserializeOwned + Send + 'static,
        Res: Serialize + Send + 'static,
        F: Fn(Request, Req) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Res, GrpcError>> + Send + 'static,
    {
        let method = method.into();
        let f = Arc::new(f);
        let handler: CtxHandler = Arc::new(move |http_req, body| {
            let f = Arc::clone(&f);
            Box::pin(async move {
                let req: Req =
                    serde_json::from_slice(&body).map_err(|e| GrpcError::Decode(e.to_string()))?;
                let res = f(http_req, req).await?;
                let bytes =
                    serde_json::to_vec(&res).map_err(|e| GrpcError::Decode(e.to_string()))?;
                Ok(Bytes::from(bytes))
            })
        });
        self.handlers
            .write()
            .unwrap()
            .insert(method, Handler::Ctx(handler));
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
        match handler {
            Handler::Body(h) => h(body).await,
            Handler::Ctx(h) => {
                let req = Request::new(http::Method::POST, format!("/{method}"));
                h(req, body).await
            }
        }
    }

    pub async fn invoke_with_request(
        &self,
        method: &str,
        http_req: Request,
        body: Bytes,
    ) -> Result<Bytes, GrpcError> {
        let handler = self
            .handlers
            .read()
            .unwrap()
            .get(method)
            .cloned()
            .ok_or_else(|| GrpcError::NotFound(method.to_string()))?;
        match handler {
            Handler::Body(h) => h(body).await,
            Handler::Ctx(h) => h(http_req, body).await,
        }
    }

    pub async fn invoke<Req, Res>(&self, method: &str, req: &Req) -> Result<Res, GrpcError>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        let body =
            Bytes::from(serde_json::to_vec(req).map_err(|e| GrpcError::Decode(e.to_string()))?);
        let out = self.invoke_raw(method, body).await?;
        serde_json::from_slice(&out).map_err(|e| GrpcError::Decode(e.to_string()))
    }

    pub fn methods(&self) -> Vec<String> {
        self.handlers.read().unwrap().keys().cloned().collect()
    }
}
