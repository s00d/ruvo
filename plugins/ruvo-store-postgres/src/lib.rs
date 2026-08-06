//! Postgres-backed [`KvStore`](ruvo_store::KvStore) via raw SQL on [`sqlx::PgPool`].

use bytes::Bytes;
use chrono::{DateTime, Utc};
use ruvo_store::{BoxFuture, KvStore};
use sqlx::PgPool;
use std::time::{Duration, SystemTime};

/// Key-value store on table `ruvo_kv`.
#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Build from a Ruvo [`ruvo_db::DbPool`] (clones the underlying sqlx pool).
    pub async fn from_db_pool(pool: &ruvo_db::DbPool) -> Result<Self, ruvo_db::DbError> {
        Ok(Self::new(pool.postgres_pool().await?))
    }

    /// `CREATE TABLE IF NOT EXISTS ruvo_kv(...)`.
    pub async fn ensure_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ruvo_kv (
                key text PRIMARY KEY,
                value bytea NOT NULL,
                expires_at timestamptz
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn expires_at(ttl: Option<Duration>) -> Option<DateTime<Utc>> {
    ttl.map(|d| {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        DateTime::<Utc>::from_timestamp(
            (now + d).as_secs() as i64,
            (now + d).subsec_nanos(),
        )
        .unwrap_or_else(Utc::now)
    })
}

impl KvStore for PostgresStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>> {
        Box::pin(async move {
            let row: Option<(Vec<u8>,)> = sqlx::query_as(
                r#"
                SELECT value FROM ruvo_kv
                WHERE key = $1
                  AND (expires_at IS NULL OR expires_at > NOW())
                "#,
            )
            .bind(key)
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
                VALUES ($1, $2, $3)
                ON CONFLICT (key) DO UPDATE SET
                    value = EXCLUDED.value,
                    expires_at = EXCLUDED.expires_at
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
            let _ = sqlx::query("DELETE FROM ruvo_kv WHERE key = $1")
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
            let row: Option<(Vec<u8>, Option<DateTime<Utc>>)> = sqlx::query_as(
                r#"
                SELECT value, expires_at FROM ruvo_kv
                WHERE key = $1
                FOR UPDATE
                "#,
            )
            .bind(key)
            .fetch_optional(&mut *tx)
            .await
            .ok()
            .flatten();

            let now = Utc::now();
            let cur = match &row {
                Some((_, Some(exp))) if *exp <= now => 0i64,
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
                VALUES ($1, $2, $3)
                ON CONFLICT (key) DO UPDATE SET
                    value = EXCLUDED.value,
                    expires_at = EXCLUDED.expires_at
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
            sqlx::query("DELETE FROM ruvo_kv WHERE key LIKE $1")
                .bind(&pattern)
                .execute(&self.pool)
                .await
                .map(|r| r.rows_affected())
                .unwrap_or(0)
        })
    }
}
