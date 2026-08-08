//! Framework-style test harness for Sova.
//!
//! - [`SqliteTestDb`] — tempfile sqlite + migrate
//! - [`TestApp`] — App bootstrap with Db + plugins
//! - [`ResponseAssert`] / [`assert_json_snapshot`] — uniform response checks
//!
//! Auth / notifications `acting_as` helpers stay in those plugins' test utils so
//! this crate can publish to crates.io without a dependency cycle.

mod app;
mod assert;
mod sqlite;

pub use app::{TestApp, TestAppBuilder};
pub use assert::with_json_redactions;
pub use sova_core::ResponseAssert;
pub use sqlite::{apply_migrations, SqliteTestDb};
