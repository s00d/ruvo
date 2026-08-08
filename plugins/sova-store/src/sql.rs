//! SQL-backed [`KvStore`] on the shared [`sova_db::DbPool`] (postgres / sqlite / mysql).

use crate::{BoxFuture, KvStore};
use bytes::Bytes;
use sova_db::DbPool;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait, Value,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Key-value store on table `sova_kv`, using the app [`DbPool`].
#[derive(Clone)]
pub struct SqlStore {
    pool: DbPool,
    schema_ready: Arc<AtomicBool>,
}

impl SqlStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            schema_ready: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Bind to Sova [`DbPool`]. Schema is created on first use (pool may connect later).
    pub fn from_db_pool(pool: &DbPool) -> Self {
        Self::new(pool.clone())
    }

    pub async fn ensure_schema(&self) -> Result<(), DbErr> {
        let conn = self.pool.get().await.map_err(|e| e.0)?;
        self.ensure_schema_on(&conn).await?;
        self.schema_ready.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn ensure_schema_on(&self, conn: &DatabaseConnection) -> Result<(), DbErr> {
        let backend = conn.get_database_backend();
        let ddl = match backend {
            DbBackend::Postgres => {
                r#"
                CREATE TABLE IF NOT EXISTS sova_kv (
                    key text PRIMARY KEY,
                    value bytea NOT NULL,
                    expires_at text
                )
                "#
            }
            DbBackend::MySql => {
                r#"
                CREATE TABLE IF NOT EXISTS sova_kv (
                    `key` varchar(768) PRIMARY KEY,
                    value longblob NOT NULL,
                    expires_at varchar(40) NULL
                )
                "#
            }
            _ => {
                r#"
                CREATE TABLE IF NOT EXISTS sova_kv (
                    key TEXT PRIMARY KEY,
                    value BLOB NOT NULL,
                    expires_at TEXT
                )
                "#
            }
        };
        conn.execute_unprepared(ddl).await?;
        let expires_idx = match backend {
            DbBackend::MySql => "CREATE INDEX sova_kv_expires_at_idx ON sova_kv (expires_at)",
            _ => "CREATE INDEX IF NOT EXISTS sova_kv_expires_at_idx ON sova_kv (expires_at)",
        };
        let _ = conn.execute_unprepared(expires_idx).await;
        Ok(())
    }

    async fn conn(&self) -> Result<DatabaseConnection, DbErr> {
        let conn = self.pool.get().await.map_err(|e| e.0)?;
        if !self.schema_ready.load(Ordering::Acquire) {
            self.ensure_schema_on(&conn).await?;
            self.schema_ready.store(true, Ordering::Release);
        }
        Ok(conn)
    }
}

fn chrono_now() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::<chrono::Utc>::from(SystemTime::now())
}

fn expires_at(ttl: Option<Duration>) -> Option<String> {
    ttl.map(|d| {
        let t = SystemTime::now()
            .checked_add(d)
            .unwrap_or_else(SystemTime::now);
        chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()
    })
}

fn expired(exp: &Option<String>) -> bool {
    match exp {
        None => false,
        Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
            Ok(dt) => dt.with_timezone(&chrono::Utc) <= chrono_now(),
            Err(_) => true,
        },
    }
}

fn key_col(backend: DbBackend) -> &'static str {
    match backend {
        DbBackend::MySql => "`key`",
        _ => "key",
    }
}

fn upsert_sql(backend: DbBackend) -> String {
    let k = key_col(backend);
    match backend {
        DbBackend::MySql => format!(
            r#"
            INSERT INTO sova_kv ({k}, value, expires_at)
            VALUES ($1, $2, $3)
            ON DUPLICATE KEY UPDATE
                value = VALUES(value),
                expires_at = VALUES(expires_at)
            "#
        ),
        DbBackend::Postgres => format!(
            r#"
            INSERT INTO sova_kv ({k}, value, expires_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (key) DO UPDATE SET
                value = EXCLUDED.value,
                expires_at = EXCLUDED.expires_at
            "#
        ),
        _ => format!(
            r#"
            INSERT INTO sova_kv ({k}, value, expires_at)
            VALUES ($1, $2, $3)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                expires_at = excluded.expires_at
            "#
        ),
    }
}

fn select_sql(backend: DbBackend) -> String {
    let k = key_col(backend);
    format!("SELECT value, expires_at FROM sova_kv WHERE {k} = $1")
}

fn delete_sql(backend: DbBackend) -> String {
    let k = key_col(backend);
    format!("DELETE FROM sova_kv WHERE {k} = $1")
}

fn delete_prefix_sql(backend: DbBackend) -> String {
    let k = key_col(backend);
    format!("DELETE FROM sova_kv WHERE {k} LIKE $1")
}

fn val_bytes(b: &[u8]) -> Value {
    Value::Bytes(Some(b.to_vec()))
}

fn val_str(s: Option<&str>) -> Value {
    match s {
        Some(s) => Value::String(Some(s.to_string())),
        None => Value::String(None),
    }
}

fn val_key(key: &str) -> Value {
    Value::String(Some(key.to_string()))
}

impl KvStore for SqlStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>> {
        Box::pin(async move {
            let conn = match self.conn().await {
                Ok(c) => c,
                Err(_) => return None,
            };
            let backend = conn.get_database_backend();
            let stmt = Statement::from_sql_and_values(
                backend,
                select_sql(backend),
                [val_key(key)],
            );
            let row = match conn.query_one_raw(stmt).await {
                Ok(r) => r,
                Err(_) => return None,
            };
            let row = row?;
            let exp: Option<String> = row.try_get_by("expires_at").ok().flatten();
            if expired(&exp) {
                let _ = conn
                    .execute_raw(Statement::from_sql_and_values(
                        backend,
                        delete_sql(backend),
                        [val_key(key)],
                    ))
                    .await;
                return None;
            }
            let v: Vec<u8> = row.try_get_by("value").ok()?;
            Some(Bytes::from(v))
        })
    }

    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let Ok(conn) = self.conn().await else {
                return;
            };
            let backend = conn.get_database_backend();
            let exp = expires_at(ttl);
            let _ = conn
                .execute_raw(Statement::from_sql_and_values(
                    backend,
                    upsert_sql(backend),
                    [val_key(key), val_bytes(val.as_ref()), val_str(exp.as_deref())],
                ))
                .await;
        })
    }

    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let Ok(conn) = self.conn().await else {
                return;
            };
            let backend = conn.get_database_backend();
            let _ = conn
                .execute_raw(Statement::from_sql_and_values(
                    backend,
                    delete_sql(backend),
                    [val_key(key)],
                ))
                .await;
        })
    }

    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let Ok(conn) = self.conn().await else {
                return 0;
            };
            let backend = conn.get_database_backend();
            let exp = expires_at(ttl);
            let key = key.to_string();
            let result = conn
                .transaction::<_, u64, DbErr>(|txn| {
                    let key = key.clone();
                    let exp = exp.clone();
                    Box::pin(async move {
                        let stmt = Statement::from_sql_and_values(
                            backend,
                            select_sql(backend),
                            [val_key(&key)],
                        );
                        let row = txn.query_one_raw(stmt).await?;
                        let cur = match row {
                            Some(row) => {
                                let e: Option<String> =
                                    row.try_get_by("expires_at").ok().flatten();
                                if expired(&e) {
                                    0i64
                                } else {
                                    let v: Vec<u8> = row.try_get_by("value").unwrap_or_default();
                                    String::from_utf8(v)
                                        .ok()
                                        .and_then(|s| s.parse().ok())
                                        .unwrap_or(0)
                                }
                            }
                            None => 0,
                        };
                        let next = (cur + by).max(0) as u64;
                        let bytes = next.to_string().into_bytes();
                        txn.execute_raw(Statement::from_sql_and_values(
                            backend,
                            upsert_sql(backend),
                            [val_key(&key), val_bytes(&bytes), val_str(exp.as_deref())],
                        ))
                        .await?;
                        Ok(next)
                    })
                })
                .await;
            result.unwrap_or(0)
        })
    }

    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let Ok(conn) = self.conn().await else {
                return 0;
            };
            let backend = conn.get_database_backend();
            let pattern = format!("{prefix}%");
            conn.execute_raw(Statement::from_sql_and_values(
                backend,
                delete_prefix_sql(backend),
                [Value::String(Some(pattern))],
            ))
            .await
            .map(|r| r.rows_affected())
            .unwrap_or(0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use std::sync::Arc;

    async fn mem_store() -> SqlStore {
        let conn = Database::connect("sqlite::memory:").await.unwrap();
        let pool = DbPool::new();
        pool.set(conn).await;
        let store = SqlStore::from_db_pool(&pool);
        store.ensure_schema().await.unwrap();
        store
    }

    #[tokio::test]
    async fn set_get_remove() {
        let store = mem_store().await;
        store.set("a", Bytes::from_static(b"1"), None).await;
        assert_eq!(store.get("a").await.unwrap(), Bytes::from_static(b"1"));
        store.remove("a").await;
        assert!(store.get("a").await.is_none());
    }

    #[tokio::test]
    async fn incr_and_prefix() {
        let store = Arc::new(mem_store().await);
        assert_eq!(store.incr("n", 2, None).await, 2);
        assert_eq!(store.incr("n", 3, None).await, 5);
        store.set("p:1", Bytes::from_static(b"x"), None).await;
        store.set("p:2", Bytes::from_static(b"y"), None).await;
        store.set("q:1", Bytes::from_static(b"z"), None).await;
        assert_eq!(store.clear_prefix("p:").await, 2);
        assert!(store.get("p:1").await.is_none());
        assert!(store.get("q:1").await.is_some());
    }
}
