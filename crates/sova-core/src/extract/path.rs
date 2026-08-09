use super::{params_as, FromRequestParts};
use crate::error::Result;
use crate::request::Request;
use serde::de::DeserializeOwned;

/// Path parameters deserialized into `T` (e.g. `struct Id { id: String }` for `/:id`).
#[derive(Debug, Clone)]
pub struct Path<T>(pub T);

impl<T> Path<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Path<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned + Send> FromRequestParts for Path<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        Ok(Path(params_as(req)?))
    }
}
