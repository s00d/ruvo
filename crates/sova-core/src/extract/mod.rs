//! Typed request extractors (`Path`, `Query`, `Json`, …) for [`IntoHandler`](crate::handler::IntoHandler).
//!
//! Existing `async fn(req: Request)` handlers keep working. Extractor handlers use separate
//! marker impls so they do not conflict with `Fn(Request)`.

mod form;
mod json;
mod path;
mod query;
mod state;

pub use form::Form;
pub use json::Json;
pub use path::Path;
pub use query::Query;
pub use state::{Extension, State};

use crate::error::{Error, Result};
use crate::request::Request;
use std::future::Future;
use std::pin::Pin;

/// Boxed future tied to the request borrow (body extractors).
pub type ExtractFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Extract a value from request parts (no body).
pub trait FromRequestParts: Sized + Send {
    fn from_request_parts(req: &Request) -> Result<Self>;
}

/// Extract a value from the request (may read the body).
pub trait FromRequest: Sized + Send {
    fn from_request(req: &mut Request) -> ExtractFuture<'_, Result<Self>>;
}

impl<T> FromRequest for T
where
    T: FromRequestParts + 'static,
{
    fn from_request(req: &mut Request) -> ExtractFuture<'_, Result<Self>> {
        Box::pin(async move { T::from_request_parts(req) })
    }
}

/// Deserialize path params (string map) into `T` via JSON intermediate.
pub(crate) fn params_as<T: serde::de::DeserializeOwned>(req: &Request) -> Result<T> {
    let map: serde_json::Map<String, serde_json::Value> = req
        .params
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();
    serde_json::from_value(serde_json::Value::Object(map)).map_err(|e| {
        Error::BadRequest(format!("path params: {e}"))
    })
}
