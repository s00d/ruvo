use super::FromRequestParts;
use crate::error::Result;
use crate::request::Request;
use serde::de::DeserializeOwned;

/// Query string deserialized into `T`.
#[derive(Debug, Clone)]
pub struct Query<T>(pub T);

impl<T> Query<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Query<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned + Send> FromRequestParts for Query<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        Ok(Query(req.query_as()?))
    }
}
