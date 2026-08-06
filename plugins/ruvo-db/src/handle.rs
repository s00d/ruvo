use crate::DbError;
use ruvo_core::Request;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, ExecResult,
    QueryResult, Statement,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared pool handle filled during `on_startup`.
#[derive(Clone, Default)]
pub struct DbPool {
    inner: Arc<RwLock<Option<DatabaseConnection>>>,
}

impl DbPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set(&self, conn: DatabaseConnection) {
        *self.inner.write().await = Some(conn);
    }

    pub async fn get(&self) -> Result<DatabaseConnection, DbError> {
        self.inner
            .read()
            .await
            .clone()
            .ok_or_else(|| DbError(DbErr::Custom("database not connected".into())))
    }

    pub async fn clear(&self) {
        if let Some(conn) = self.inner.write().await.take() {
            drop(conn);
        }
    }

    /// Clone the sqlx Postgres pool (for raw backends).
    pub async fn postgres_pool(&self) -> Result<sqlx::PgPool, DbError> {
        let conn = self.get().await?;
        Ok(conn.get_postgres_connection_pool().clone())
    }
}

/// Request-scoped DB handle: pool connection or open transaction.
#[derive(Clone)]
pub enum DbHandle {
    Conn(DatabaseConnection),
    Tx(Arc<DatabaseTransaction>),
}

impl DbHandle {
    pub fn as_conn(&self) -> Option<&DatabaseConnection> {
        match self {
            Self::Conn(c) => Some(c),
            Self::Tx(_) => None,
        }
    }
}

#[async_trait::async_trait]
impl ConnectionTrait for DbHandle {
    fn get_database_backend(&self) -> DbBackend {
        match self {
            Self::Conn(c) => c.get_database_backend(),
            Self::Tx(t) => t.get_database_backend(),
        }
    }

    async fn execute_raw(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        match self {
            Self::Conn(c) => c.execute_raw(stmt).await,
            Self::Tx(t) => t.execute_raw(stmt).await,
        }
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        match self {
            Self::Conn(c) => c.execute_unprepared(sql).await,
            Self::Tx(t) => t.execute_unprepared(sql).await,
        }
    }

    async fn query_one_raw(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        match self {
            Self::Conn(c) => c.query_one_raw(stmt).await,
            Self::Tx(t) => t.query_one_raw(stmt).await,
        }
    }

    async fn query_all_raw(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        match self {
            Self::Conn(c) => c.query_all_raw(stmt).await,
            Self::Tx(t) => t.query_all_raw(stmt).await,
        }
    }
}

/// Convenient access to the request [`DbHandle`].
pub trait DbExt {
    fn db(&self) -> &DbHandle;
}

impl DbExt for Request {
    fn db(&self) -> &DbHandle {
        self.get::<DbHandle>()
            .expect("Db plugin is not installed (missing req.db())")
    }
}
