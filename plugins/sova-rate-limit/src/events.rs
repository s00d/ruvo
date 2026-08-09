//! Domain events emitted on the app [`EventBus`](sova_core::EventBus).

use sova_core::Event;

/// Fired before a 429 response when a rate limit is exceeded.
#[derive(Debug, Clone)]
pub struct RateLimitExceeded {
    pub key: String,
    pub limit: u64,
    pub retry_after: Option<u64>,
}

impl Event for RateLimitExceeded {
    fn name(&self) -> &'static str {
        "rate_limit.exceeded"
    }
}
