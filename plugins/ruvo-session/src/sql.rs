//! SQL-backed [`SessionStore`] (`ruvo_sessions` + `user_id` column).

use super::store::{decode, encode, SessionStore, SESSION_USER_KEY};
use ruvo_db::DbPool;
use ruvo_store::BoxFuture;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, Value,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Sessions table with indexed `user_id` (Laravel-style logout others/all).
#[derive(Clone)]
pub struct SqlSessionStore {
    pool: DbPool,
    schema_ready: Arc<AtomicBool>,
}

impl SqlSessionStore {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            schema_ready: Arc::new(AtomicBool::new(false)),
        }
    }

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
                CREATE TABLE IF NOT EXISTS ruvo_sessions (
                    id text PRIMARY KEY,
                    user_id text,
                    payload bytea NOT NULL,
                    expires_at text
                );
                CREATE INDEX IF NOT EXISTS ruvo_sessions_user_id_idx
                    ON ruvo_sessions (user_id);
                CREATE INDEX IF NOT EXISTS ruvo_sessions_expires_at_idx
                    ON ruvo_sessions (expires_at);
                "#
            }
            DbBackend::MySql => {
                r#"
                CREATE TABLE IF NOT EXISTS ruvo_sessions (
                    id varchar(128) PRIMARY KEY,
                    user_id varchar(64) NULL,
                    payload longblob NOT NULL,
                    expires_at varchar(40) NULL,
                    INDEX ruvo_sessions_user_id_idx (user_id),
                    INDEX ruvo_sessions_expires_at_idx (expires_at)
                )
                "#
            }
            _ => {
                r#"
                CREATE TABLE IF NOT EXISTS ruvo_sessions (
                    id TEXT PRIMARY KEY,
                    user_id TEXT,
                    payload BLOB NOT NULL,
                    expires_at TEXT
                );
                CREATE INDEX IF NOT EXISTS ruvo_sessions_user_id_idx
                    ON ruvo_sessions (user_id);
                CREATE INDEX IF NOT EXISTS ruvo_sessions_expires_at_idx
                    ON ruvo_sessions (expires_at);
                "#
            }
        };
        // Postgres/SQLite may send multiple statements; execute separately when needed.
        for stmt in ddl.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            conn.execute_unprepared(stmt).await?;
        }
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

fn expires_at(ttl: Duration) -> String {
    let t = SystemTime::now()
        .checked_add(ttl)
        .unwrap_or_else(SystemTime::now);
    chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()
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

fn upsert_sql(backend: DbBackend) -> &'static str {
    match backend {
        DbBackend::MySql => {
            r#"
            INSERT INTO ruvo_sessions (id, user_id, payload, expires_at)
            VALUES ($1, $2, $3, $4)
            ON DUPLICATE KEY UPDATE
                user_id = VALUES(user_id),
                payload = VALUES(payload),
                expires_at = VALUES(expires_at)
            "#
        }
        DbBackend::Postgres => {
            r#"
            INSERT INTO ruvo_sessions (id, user_id, payload, expires_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                payload = EXCLUDED.payload,
                expires_at = EXCLUDED.expires_at
            "#
        }
        _ => {
            r#"
            INSERT INTO ruvo_sessions (id, user_id, payload, expires_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT(id) DO UPDATE SET
                user_id = excluded.user_id,
                payload = excluded.payload,
                expires_at = excluded.expires_at
            "#
        }
    }
}

impl SessionStore for SqlSessionStore {
    fn load<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Option<HashMap<String, String>>> {
        Box::pin(async move {
            let conn = match self.conn().await {
                Ok(c) => c,
                Err(_) => return None,
            };
            let backend = conn.get_database_backend();
            let stmt = Statement::from_sql_and_values(
                backend,
                "SELECT payload, expires_at FROM ruvo_sessions WHERE id = $1",
                [val_key(id)],
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
                        "DELETE FROM ruvo_sessions WHERE id = $1",
                        [val_key(id)],
                    ))
                    .await;
                return None;
            }
            let v: Vec<u8> = row.try_get_by("payload").ok()?;
            Some(decode(&v))
        })
    }

    fn save<'a>(
        &'a self,
        id: &'a str,
        data: &'a HashMap<String, String>,
        ttl: Duration,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let Ok(conn) = self.conn().await else {
                return;
            };
            let backend = conn.get_database_backend();
            let uid = data
                .get(SESSION_USER_KEY)
                .filter(|s| !s.is_empty())
                .map(String::as_str);
            let exp = expires_at(ttl);
            let payload = encode(data);
            let _ = conn
                .execute_raw(Statement::from_sql_and_values(
                    backend,
                    upsert_sql(backend),
                    [
                        val_key(id),
                        val_str(uid),
                        val_bytes(payload.as_ref()),
                        val_str(Some(&exp)),
                    ],
                ))
                .await;
        })
    }

    fn destroy<'a>(&'a self, id: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let Ok(conn) = self.conn().await else {
                return;
            };
            let backend = conn.get_database_backend();
            let _ = conn
                .execute_raw(Statement::from_sql_and_values(
                    backend,
                    "DELETE FROM ruvo_sessions WHERE id = $1",
                    [val_key(id)],
                ))
                .await;
        })
    }

    fn destroy_user<'a>(
        &'a self,
        user_id: &'a str,
        keep_sid: Option<&'a str>,
    ) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            if user_id.is_empty() {
                return 0;
            }
            let Ok(conn) = self.conn().await else {
                return 0;
            };
            let backend = conn.get_database_backend();
            let result = match keep_sid {
                Some(keep) => {
                    conn.execute_raw(Statement::from_sql_and_values(
                        backend,
                        "DELETE FROM ruvo_sessions WHERE user_id = $1 AND id <> $2",
                        [val_key(user_id), val_key(keep)],
                    ))
                    .await
                }
                None => {
                    conn.execute_raw(Statement::from_sql_and_values(
                        backend,
                        "DELETE FROM ruvo_sessions WHERE user_id = $1",
                        [val_key(user_id)],
                    ))
                    .await
                }
            };
            result.map(|r| r.rows_affected()).unwrap_or(0)
        })
    }
}
