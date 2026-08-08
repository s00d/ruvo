//! Notifications-only `acting_as_id` (no auth feature).

use ruvo_core::TestClient;
use ruvo_notifications::NotificationUser;

/// Inject [`NotificationUser`] on every request.
pub trait ActingAs {
    fn acting_as_id(&self, user_id: i64);
    fn logout(&self);
}

impl ActingAs for TestClient {
    fn acting_as_id(&self, user_id: i64) {
        self.clear_request_hooks();
        self.on_request(move |req| {
            req.set(NotificationUser(user_id));
        });
    }

    fn logout(&self) {
        self.clear_request_hooks();
    }
}
