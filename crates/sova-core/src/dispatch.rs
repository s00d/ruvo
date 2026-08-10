//! In-process dispatch handle (filled when the server starts).

use crate::app::AppInner;
use crate::handler::BoxFuture;
use crate::request::Request;
use crate::response::Response;
use std::sync::{Arc, OnceLock};

type DispatchFn = Arc<dyn Fn(Request) -> BoxFuture<Response> + Send + Sync>;

struct Inner {
    dispatch: DispatchFn,
}

/// Handle for dispatching requests inside the running app (DevTools console, tests).
#[derive(Clone, Default)]
pub struct AppDispatch {
    slot: Arc<OnceLock<Inner>>,
}

impl AppDispatch {
    /// Dispatch if the server has started; `None` before listen.
    pub fn try_dispatch(&self, req: Request) -> Option<BoxFuture<Response>> {
        Some((self.slot.get()?.dispatch)(req))
    }

    pub(crate) fn install_if_needed(&self, inner: &AppInner) {
        if self.slot.get().is_some() {
            return;
        }
        let _ = self.install(inner);
    }

    pub(crate) fn install(&self, inner: &AppInner) -> bool {
        if self.slot.get().is_some() {
            return false;
        }
        let compiled = Arc::clone(&inner.compiled);
        let state = inner.state();
        let dispatch: DispatchFn = Arc::new(move |mut req: Request| {
            req.state = Arc::clone(&state);
            let compiled = Arc::clone(&compiled);
            Box::pin(async move { compiled.dispatch(req).await })
        });
        self.slot.set(Inner { dispatch }).is_ok()
    }
}
