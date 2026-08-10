//! Framework-style test harness for Sova.
//!
//! - [`SqliteTestDb`] — tempfile sqlite + migrate
//! - [`TestApp`] — App bootstrap with Db + plugins
//! - [`ResponseAssert`] / [`assert_json_snapshot`] — uniform response checks
//!
//! **Apps:** prefer `sova = { features = ["testing"] }` — the facade re-exports this crate.
//! **Plugins:** depend on `sova-testing` / `sova-core` directly (avoid the facade).
//!
//! Auth helpers (`acting_as`, user factories) live in [`sova_auth::testing`].
//! Notification `acting_as_id` lives in [`sova_notifications::testing`].
//!
//! ```ignore
//! use sova::{ResponseAssert, TestApp, TestClient, assert_json_snapshot};
//! use sova_notifications::testing::ActingAs;
//!
//! let (_db, app) = TestApp::builder()
//!     .migrator::<NotificationsMigrator>()
//!     .install(Notifications::new().mount("/notifications"))
//!     .build()
//!     .await;
//! let c = TestClient::tracked(app).await.unwrap();
//! c.acting_as_id(1);
//! let res = c.get("/notifications").await;
//! res.assert_status(200);
//! assert_json_snapshot!("inbox", res.json_value());
//! ```

mod app;
mod assert;
mod sqlite;

pub use app::{TestApp, TestAppBuilder};
pub use assert::with_json_redactions;
pub use sova_core::ResponseAssert;
pub use sqlite::{apply_migrations, SqliteTestDb};

/// Re-export so [`assert_json_snapshot`] can resolve `$crate::insta` from dependents.
pub use insta;
