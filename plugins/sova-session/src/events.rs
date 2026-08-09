//! Domain events emitted on the app [`EventBus`](sova_core::EventBus).

use sova_core::Event;

/// Fired when [`crate::Session::regenerate`] issues a new session id.
#[derive(Debug, Clone)]
pub struct SessionRegenerated {
    pub had_user: bool,
}

impl Event for SessionRegenerated {
    fn name(&self) -> &'static str {
        "session.regenerated"
    }
}

/// Fired after [`crate::SessionExt::logout_all_sessions`] destroys sessions.
#[derive(Debug, Clone)]
pub struct SessionLogoutAll {
    pub user_id: String,
    pub count: u64,
}

impl Event for SessionLogoutAll {
    fn name(&self) -> &'static str {
        "session.logout_all"
    }
}
