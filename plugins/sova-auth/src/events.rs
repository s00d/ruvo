//! Domain events emitted on the app [`EventBus`](sova_core::EventBus).

use sova_core::Event;

/// Fired after a successful registration (before/with login finish).
#[derive(Debug, Clone)]
pub struct UserRegistered {
    pub user_id: i64,
    pub email: String,
}

impl Event for UserRegistered {
    fn name(&self) -> &'static str {
        "auth.user_registered"
    }
}

/// Fired after a successful interactive login (session established).
#[derive(Debug, Clone)]
pub struct UserLoggedIn {
    pub user_id: i64,
    pub email: String,
}

impl Event for UserLoggedIn {
    fn name(&self) -> &'static str {
        "auth.user_logged_in"
    }
}
