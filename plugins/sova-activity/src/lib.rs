//! Laravel-style activity / audit log for Sova.
//!
//! ```ignore
//! app.install(Db::from_env().migrations::<ActivityMigrator>());
//! // or compose: AuthMigrator::migrations() + ActivityMigrator::migrations()
//! app.install(Activity::new().mount("/activity"));
//! req.log_activity("note.created", "note", id, json!({})).await;
//! ```

mod entity;
mod list;
mod migration;
mod plugin;
mod record;

pub use list::{list_activity, ActivityFilter, ActivityRow};
pub use migration::ActivityMigrator;
pub use plugin::Activity;
pub use record::{ActivityActor, ActivityEntry, ActivityExt, ActivityLog};
