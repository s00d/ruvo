//! Framework-style test harness for Sova.
//!
//! - [`SqliteTestDb`] — tempfile sqlite + migrate
//! - [`TestApp`] — App bootstrap with Db + plugins
//! - [`ActingAs`] — inject auth / notification user on [`TestClient`]
//! - [`ResponseAssert`] / [`assert_json_snapshot`] — uniform response checks
//!
//! ```ignore
//! use sova_testing::{ActingAs, ResponseAssert, TestApp, assert_json_snapshot};
//!
//! let (_db, app) = TestApp::builder()
//!     .migrator::<NotificationsMigrator>()
//!     .install(Notifications::new().mount("/notifications"))
//!     .build()
//!     .await;
//! let c = TestClient::tracked(app).unwrap();
//! c.acting_as_id(1);
//! let res = c.get("/notifications").await;
//! res.assert_status(200);
//! assert_json_snapshot!("inbox", res.json_value());
//! ```

mod app;
mod assert;
mod sqlite;

#[cfg(feature = "auth")]
mod acting;
#[cfg(feature = "auth")]
mod factory;

#[cfg(all(not(feature = "auth"), feature = "notifications"))]
mod acting_id;

pub use app::{TestApp, TestAppBuilder};
pub use assert::with_json_redactions;
pub use sova_core::ResponseAssert;
pub use sqlite::{apply_migrations, SqliteTestDb};

#[cfg(feature = "auth")]
pub use acting::ActingAs;
#[cfg(feature = "auth")]
pub use factory::{ensure_permission, ensure_role, UserFactory};

#[cfg(all(not(feature = "auth"), feature = "notifications"))]
pub use acting_id::ActingAs;
