use crate::handle::DbHandle;
use crate::DbError;
use sea_orm::{Database, DatabaseConnection, DbErr, TransactionTrait};
use std::sync::Arc;

/// Isolated test connection: pool + open transaction that rolls back on [`TestDb::rollback`].
pub struct TestDb {
    _conn: DatabaseConnection,
    handle: DbHandle,
    tx: Option<Arc<sea_orm::DatabaseTransaction>>,
}

impl TestDb {
    pub fn db(&self) -> &DbHandle {
        &self.handle
    }

    pub async fn rollback(mut self) -> Result<(), DbError> {
        if let Some(arc) = self.tx.take() {
            match Arc::try_unwrap(arc) {
                Ok(tx) => tx.rollback().await.map_err(DbError)?,
                Err(_) => {
                    return Err(DbError(DbErr::Custom(
                        "test transaction still held".into(),
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Connect via `DATABASE_URL` and begin a transaction for test isolation.
pub async fn test_db() -> Result<TestDb, DbError> {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| DbError(DbErr::Custom("DATABASE_URL is not set".into())))?;
    let conn = Database::connect(&url).await.map_err(DbError)?;
    let tx = conn.begin().await.map_err(DbError)?;
    let arc = Arc::new(tx);
    Ok(TestDb {
        _conn: conn,
        handle: DbHandle::Tx(Arc::clone(&arc)),
        tx: Some(arc),
    })
}
