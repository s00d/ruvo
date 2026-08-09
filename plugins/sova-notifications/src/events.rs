//! Domain events emitted on the app [`EventBus`](sova_core::EventBus).

use sova_core::Event;

/// Fired at the end of [`crate::Notify::send`].
#[derive(Debug, Clone)]
pub struct NotificationSent {
    pub channel: String,
    pub event: String,
    pub recipients: Vec<i64>,
}

impl Event for NotificationSent {
    fn name(&self) -> &'static str {
        "notifications.sent"
    }
}
