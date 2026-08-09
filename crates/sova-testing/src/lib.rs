//! Framework-style test harness for Sova.
//!
//! - [`SqliteTestDb`] — tempfile sqlite + migrate
//! - [`TestApp`] — App bootstrap with Db + plugins
//! - [`ResponseAssert`] / [`assert_json_snapshot`] — uniform response checks
//!
//! Auth helpers (`acting_as`, user factories) live in [`sova_auth::testing`].
//! Notification `acting_as_id` lives in [`sova_notifications::testing`].
//!
//! ```ignore
//! use sova_testing::{ResponseAssert, TestApp, assert_json_snapshot};
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
