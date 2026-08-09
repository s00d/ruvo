//! Database notifications with named channels, ACL, optional WS / mail.
//!
//! ```ignore
//! app.install(
//!   Notifications::new()
//!     .channel(Channel::new("orders").publish("notifications.orders.publish"))
//!     .mount("/notifications")
//!     .guard(Fortify::guard())
//! );
//! Notify::to(user_id).channel("orders").event("order.shipped").title("Shipped").send(&req).await?;
//! ```

mod channel;
mod entity;
mod events;
mod http;
mod list;
mod migration;
mod notify;
mod plugin;

#[cfg(feature = "auth")]
mod audience;

#[cfg(feature = "ws")]
mod ws;

#[cfg(feature = "templates")]
mod templates;

pub use channel::{Channel, Via};
pub use events::NotificationSent;
pub use list::{
    list_notifications, mark_all_read, mark_read, unread_count, NotificationFilter, NotificationRow,
};
pub use migration::NotificationsMigrator;
pub use notify::{NotificationService, NotificationUser, Notify, NotifyExt};
pub use plugin::Notifications;

#[cfg(feature = "testing")]
pub mod testing;

#[cfg(feature = "templates")]
pub use templates::{preload_unread, UnreadCount};
