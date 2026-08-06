//! SQLite-backed [`KvStore`](ruvo_store::KvStore) via raw SQL on [`sqlx::SqlitePool`].

use bytes::Bytes;
use ruvo_store::{BoxFuture, KvStore};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Key-value store on table `ruvo_kv`.
#[derive(Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Open (or create) a SQLite database at `path`.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, sqlx::Error> {
        let url = format!("sqlite:{}?mode=rwc", path.as_ref().display());
        let pool = SqlitePool::connect(&url).await?;
        let store = Self::new(pool);
        store.ensure_schema().await?;
        Ok(store)
    }

    /// In-memory database (tests / ephemeral).
    pub async fn memory() -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        let store = Self::new(pool);
        store.ensure_schema().await?;
        Ok(store)
    }

    pub async fn ensure_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ruvo_kv (
                key TEXT PRIMARY KEY,
                value BLOB NOT NULL,
                expires_at INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn expires_at(ttl: Option<Duration>) -> Option<i64> {
    ttl.map(|d| now_secs() + d.as_secs() as i64)
}

impl KvStore for SqliteStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>> {
        Box::pin(async move {
            let now = now_secs();
            let row: Option<(Vec<u8>,)> = sqlx::query_as(
                r#"
                SELECT value FROM ruvo_kv
                WHERE key = ?
                  AND (expires_at IS NULL OR expires_at > ?)
                "#,
            )
            .bind(key)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();
            row.map(|(v,)| Bytes::from(v))
        })
    }

    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let exp = expires_at(ttl);
            let _ = sqlx::query(
                r#"
                INSERT INTO ruvo_kv (key, value, expires_at)
                VALUES (?, ?, ?)
                ON CONFLICT(key) DO UPDATE SET
                    value = excluded.value,
                    expires_at = excluded.expires_at
                "#,
            )
            .bind(key)
            .bind(val.as_ref())
            .bind(exp)
            .execute(&self.pool)
            .await;
        })
    }

    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let _ = sqlx::query("DELETE FROM ruvo_kv WHERE key = ?")
                .bind(key)
                .execute(&self.pool)
                .await;
        })
    }

    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let exp = expires_at(ttl);
            let mut tx = match self.pool.begin().await {
                Ok(tx) => tx,
                Err(_) => return 0,
            };
            let row: Option<(Vec<u8>, Option<i64>)> = sqlx::query_as(
                "SELECT value, expires_at FROM ruvo_kv WHERE key = ?",
            )
            .bind(key)
            .fetch_optional(&mut *tx)
            .await
            .ok()
            .flatten();

            let now = now_secs();
            let cur = match &row {
                Some((_, Some(e))) if *e <= now => 0i64,
                Some((val, _)) => std::str::from_utf8(val)
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                None => 0,
            };
            let next = (cur + by).max(0) as u64;
            let bytes = next.to_string().into_bytes();

            let ok = sqlx::query(
                r#"
                INSERT INTO ruvo_kv (key, value, expires_at)
                VALUES (?, ?, ?)
                ON CONFLICT(key) DO UPDATE SET
                    value = excluded.value,
                    expires_at = excluded.expires_at
                "#,
            )
            .bind(key)
            .bind(&bytes)
            .bind(exp)
            .execute(&mut *tx)
            .await
            .is_ok();

            if ok {
                let _ = tx.commit().await;
                next
            } else {
                let _ = tx.rollback().await;
                0
            }
        })
    }

    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let pattern = format!("{prefix}%");
            sqlx::query("DELETE FROM ruvo_kv WHERE key LIKE ?")
                .bind(&pattern)
                .execute(&self.pool)
                .await
                .map(|r| r.rows_affected())
                .unwrap_or(0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn set_get_remove() {
        let store = SqliteStore::memory().await.unwrap();
        store.set("a", Bytes::from_static(b"1"), None).await;
        assert_eq!(store.get("a").await.unwrap(), Bytes::from_static(b"1"));
        store.remove("a").await;
        assert!(store.get("a").await.is_none());
    }

    #[tokio::test]
    async fn incr_and_prefix() {
        let store = Arc::new(SqliteStore::memory().await.unwrap());
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
