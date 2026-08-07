//! SeaORM database plugin for Ruvo (postgres / sqlite / mysql via Cargo features).
//!
//! ```ignore
//! app.install(Db::from_env().migrations::<Migrator>());
//! let u = User::find_by_id(id).one(req.db()).await?;
//! ```

mod error;
mod handle;
mod plugin;
mod test_db;
mod tx;

pub use error::DbError;
pub use handle::{DbExt, DbHandle, DbPool};
pub use plugin::Db;
pub use test_db::{test_db, TestDb};
pub use tx::transaction;

pub use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, Set, TransactionTrait,
};
pub use sea_orm_migration::prelude::{MigrationTrait, MigratorTrait};
