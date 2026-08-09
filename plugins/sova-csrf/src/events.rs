//! Domain events emitted on the app [`EventBus`](sova_core::EventBus).

use sova_core::Event;

/// Fired before a 403 response for CSRF mismatch or missing token.
#[derive(Debug, Clone)]
pub struct CsrfMismatch {
    pub method: String,
    pub path: String,
}

impl Event for CsrfMismatch {
    fn name(&self) -> &'static str {
        "csrf.mismatch"
    }
}
