//! Domain events emitted on the app [`EventBus`](sova_core::EventBus).

use sova_core::Event;

/// Fired after an API personal access token is revoked.
#[derive(Debug, Clone)]
pub struct ApiTokenRevoked {
    pub user_id: i64,
    pub token_id: i64,
}

impl Event for ApiTokenRevoked {
    fn name(&self) -> &'static str {
        "passport.api_token_revoked"
    }
}
