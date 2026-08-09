use super::{ExtractFuture, FromRequest};
use crate::error::Result;
use crate::request::Request;
use serde::de::DeserializeOwned;

/// JSON body deserialized into `T`.
///
/// Distinct from [`crate::Json`] (response wrapper) — import as `sova_core::extract::Json`.
#[derive(Debug, Clone)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned + Send + 'static> FromRequest for Json<T> {
    fn from_request(req: &mut Request) -> ExtractFuture<'_, Result<Self>> {
        Box::pin(async move {
            let value: T = req.json().await?;
            Ok(Json(value))
        })
    }
}
