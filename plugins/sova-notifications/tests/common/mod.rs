#![allow(dead_code)] // used across integration test crates

//! Shared test helpers for sova-notifications (keeps sova-testing free of plugin deps).

use sova_auth::CurrentUser;
use sova_core::TestClient;
use sova_notifications::NotificationUser;

/// Inject auth / notification user extensions on every request.
pub trait ActingAs {
    fn acting_as(&self, user: CurrentUser);
    fn acting_as_id(&self, user_id: i64);
    fn logout(&self);
}

impl ActingAs for TestClient {
    fn acting_as(&self, user: CurrentUser) {
        self.clear_request_hooks();
        let uid = user.id;
        self.on_request(move |req| {
            req.set(user.clone());
        });
        self.on_request(move |req| {
            req.set(NotificationUser(uid));
        });
    }

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
