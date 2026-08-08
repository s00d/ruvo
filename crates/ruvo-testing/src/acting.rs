//! `acting_as` helpers for Fortify [`CurrentUser`].

use ruvo_auth::CurrentUser;
use ruvo_core::TestClient;

#[cfg(feature = "notifications")]
use ruvo_notifications::NotificationUser;

/// Inject authenticated user extensions on every [`TestClient`] request.
pub trait ActingAs {
    /// Set [`CurrentUser`] (and [`NotificationUser`] when feature `notifications`).
    fn acting_as(&self, user: CurrentUser);

    /// Notifications-only: set [`NotificationUser`] by id (also available with auth).
    fn acting_as_id(&self, user_id: i64);

    /// Clear request hooks (stop acting as anyone).
    fn logout(&self);
}

impl ActingAs for TestClient {
    fn acting_as(&self, user: CurrentUser) {
        self.clear_request_hooks();
        #[cfg(feature = "notifications")]
        let uid = user.id;
        self.on_request(move |req| {
            req.set(user.clone());
        });
        #[cfg(feature = "notifications")]
        self.on_request(move |req| {
            req.set(NotificationUser(uid));
        });
    }

    fn acting_as_id(&self, user_id: i64) {
        #[cfg(feature = "notifications")]
        {
            self.clear_request_hooks();
            self.on_request(move |req| {
                req.set(NotificationUser(user_id));
            });
        }
        #[cfg(not(feature = "notifications"))]
        {
            let _ = user_id;
            panic!("acting_as_id requires ruvo-testing feature `notifications`");
        }
    }

    fn logout(&self) {
        self.clear_request_hooks();
    }
}
