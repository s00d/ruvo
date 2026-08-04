use crate::handler::BoxFuture;
use crate::response::ResponseBody;
use hyper::body::Incoming;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use std::sync::Arc;

/// Escape-hatch handler: full Hyper request in, Hyper response out.
pub type RawHandler =
    Arc<dyn Fn(HyperRequest<Incoming>) -> BoxFuture<HyperResponse<ResponseBody>> + Send + Sync>;

pub trait IntoRawHandler {
    fn into_raw_handler(self) -> RawHandler;
}

impl<F, Fut> IntoRawHandler for F
where
    F: Fn(HyperRequest<Incoming>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = HyperResponse<ResponseBody>> + Send + 'static,
{
    fn into_raw_handler(self) -> RawHandler {
        Arc::new(move |req| Box::pin(self(req)))
    }
}

impl IntoRawHandler for RawHandler {
    fn into_raw_handler(self) -> RawHandler {
        self
    }
}
