//! Template helpers (feature `templates`).

use minijinja::Value;
use sova_core::extend::{named, MwEntry};
use sova_core::{with_state, App, Request};
use sova_db::DbExt;
use sova_templates::register_per_request;

use crate::list::unread_count;
use crate::notify::NotificationUser;

pub fn register(app: &mut App) {
    register_per_request(app, "notifications_unread", |req| {
        if let Some(c) = req.get::<UnreadCount>() {
            return Value::from(c.0);
        }
        Value::from(0u64)
    });
}

/// Precomputed unread count for templates.
#[derive(Clone, Copy, Debug)]
pub struct UnreadCount(pub u64);

pub fn preload_middleware() -> MwEntry {
    named(
        "notifications-preload-unread",
        with_state((), |_s, mut req, next| async move {
            preload_unread(&mut req).await;
            next(req).await
        }),
    )
}

/// Load unread count onto the request for template helpers.
pub async fn preload_unread(req: &mut Request) {
    let uid = if let Some(u) = req.get::<NotificationUser>() {
        u.0
    } else {
        #[cfg(feature = "auth")]
        {
            use sova_auth::AuthExt;
            match req.current_user() {
                Some(u) => u.id,
                None => return,
            }
        }
        #[cfg(not(feature = "auth"))]
        {
            return;
        }
    };
    if let Ok(n) = unread_count(req.db(), uid, None).await {
        req.set(UnreadCount(n));
    }
}
