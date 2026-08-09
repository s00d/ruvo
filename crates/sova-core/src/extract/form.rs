use super::{ExtractFuture, FromRequest};
use crate::error::Result;
use crate::request::Request;
use serde::de::DeserializeOwned;

/// `application/x-www-form-urlencoded` (or multipart text fields) into `T`.
#[derive(Debug, Clone)]
pub struct Form<T>(pub T);

impl<T> Form<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Form<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned + Send + 'static> FromRequest for Form<T> {
    fn from_request(req: &mut Request) -> ExtractFuture<'_, Result<Self>> {
        Box::pin(async move {
            let value: T = req.form().await?;
            Ok(Form(value))
        })
    }
}
