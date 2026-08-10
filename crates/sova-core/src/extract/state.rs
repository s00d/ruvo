use super::FromRequestParts;
use crate::error::{Error, Result};
use crate::request::Request;
use std::sync::Arc;

/// Shared app state registered via `app.state(T)`.
#[derive(Debug, Clone)]
pub struct State<T>(pub Arc<T>);

impl<T> State<T> {
    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

impl<T> std::ops::Deref for State<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Send + Sync + 'static> FromRequestParts for State<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        req.try_state::<T>().map(State).ok_or_else(|| {
            Error::Internal(format!(
                "state `{}` is not registered — call app.state(..)",
                std::any::type_name::<T>()
            ))
        })
    }
}

/// Per-request extension (`req.set` / `req.get`).
#[derive(Debug, Clone)]
pub struct Extension<T>(pub T);

impl<T> Extension<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Extension<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone + Send + Sync + 'static> FromRequestParts for Extension<T> {
    fn from_request_parts(req: &Request) -> Result<Self> {
        req.get::<T>().cloned().map(Extension).ok_or_else(|| {
            Error::BadRequest(format!(
                "missing extension `{}`",
                std::any::type_name::<T>()
            ))
        })
    }
}
