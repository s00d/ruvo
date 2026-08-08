//! `app.ws(path, handler)` route sugar.

use std::future::Future;
use std::sync::Arc;

use sova_core::{App, Request, Router};

use crate::upgrade::upgrade_ws;
use crate::WsSession;

/// Register WebSocket routes on [`App`] / [`Router`].
pub trait WsRouteExt {
    fn ws<F, Fut>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(WsSession) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static;
}

impl WsRouteExt for Router {
    fn ws<F, Fut>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(WsSession) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler = Arc::new(handler);
        self.get(path, move |req: Request| {
            let handler = Arc::clone(&handler);
            async move {
                match upgrade_ws(req, move |session| handler(session)).await {
                    Ok(res) => res,
                    Err(res) => res,
                }
            }
        });
        self
    }
}

impl WsRouteExt for App {
    fn ws<F, Fut>(&mut self, path: &str, handler: F) -> &mut Self
    where
        F: Fn(WsSession) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Router::ws(self, path, handler);
        self
    }
}
