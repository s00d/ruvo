use crate::DbError;
use sova_core::Request;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, ExecResult,
    QueryResult, Statement,
};
use std::sync::{Arc, RwLock};

/// Shared pool handle filled during `on_startup`.
#[derive(Clone, Default)]
pub struct DbPool {
    inner: Arc<RwLock<Option<DatabaseConnection>>>,
}

impl DbPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, conn: DatabaseConnection) {
        *self.inner.write().unwrap() = Some(conn);
    }

    pub fn get(&self) -> Result<DatabaseConnection, DbError> {
        self.inner
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| DbError(DbErr::Custom("database not connected".into())))
    }

    pub fn clear(&self) {
        let _ = self.inner.write().unwrap().take();
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

    fn support_returning(&self) -> bool {
        match self {
            Self::Conn(c) => c.support_returning(),
            Self::Tx(t) => t.support_returning(),
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
