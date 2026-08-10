//! Sova HTTP context injected into GraphQL resolvers and subscriptions.

use sova_core::extend::StateMap;
use sova_core::Request;
use std::sync::Arc;

/// App state + selected HTTP metadata available in resolvers via `ctx.data::<GraphqlContext>()`.
#[derive(Clone)]
pub struct GraphqlContext {
    state: Arc<StateMap>,
    method: http::Method,
    path: String,
    authorization: Option<String>,
}

impl GraphqlContext {
    pub fn from_request(req: &Request) -> Self {
        Self {
            state: req.states(),
            method: req.method.clone(),
            path: req.path.clone(),
            authorization: req.header("authorization").map(str::to_string),
        }
    }

    pub fn states(&self) -> &Arc<StateMap> {
        &self.state
    }

    pub fn method(&self) -> &http::Method {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn authorization(&self) -> Option<&str> {
        self.authorization.as_deref()
    }

    pub fn state<T: Send + Sync + 'static>(&self) -> Arc<T> {
        self.state.get::<T>().unwrap_or_else(|| {
            panic!(
                "GraphqlContext: state `{}` not found",
                std::any::type_name::<T>()
            )
        })
    }

    pub fn try_state<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.state.get::<T>()
    }
}
