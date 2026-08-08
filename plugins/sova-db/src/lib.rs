//! SeaORM database plugin for Sova (postgres / sqlite / mysql via Cargo features).
//!
//! ```ignore
//! app.install(Db::from_env().migrations::<Migrator>());
//! let u = User::find_by_id(id).one(req.db()).await?;
//! ```

mod error;
mod handle;
mod migrate_cli;
mod page;
mod plugin;
mod test_db;
mod tx;

pub use error::DbError;
pub use handle::{DbExt, DbHandle, DbPool};
pub use migrate_cli::{parse_migrate_args, MigrateCmd};
pub use page::{Page, PageExt, PageParams, PaginateExt};
pub use plugin::Db;
pub use test_db::{test_db, TestDb};
pub use tx::transaction;

pub use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
pub use sea_orm_migration::prelude::{MigrationTrait, MigratorTrait};
