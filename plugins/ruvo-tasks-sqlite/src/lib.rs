//! SQLite [`TaskStore`](ruvo_tasks_store::TaskStore).
//!
//! Claims are serialized with a process-local mutex + `BEGIN IMMEDIATE` — SQLite
//! has no `SKIP LOCKED` and only one writer.

use bytes::Bytes;
use chrono::{DateTime, Utc};
use ruvo_tasks_store::{
    BoxFuture, EnqueueOpts, Task, TaskError, TaskStatus, TaskStore,
};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// Task queue on table `ruvo_tasks`.
#[derive(Clone)]
pub struct SqliteTaskStore {
    pool: SqlitePool,
    seq: Arc<AtomicU64>,
    /// Serializes claim so two workers in one process cannot race.
    claim_lock: Arc<Mutex<()>>,
}

impl SqliteTaskStore {
    pub fn new(pool: SqlitePool) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Self {
            pool,
            seq: Arc::new(AtomicU64::new(seed)),
            claim_lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn open(path: impl AsRef<Path>) -> Result<Self, sqlx::Error> {
        let url = format!("sqlite:{}?mode=rwc", path.as_ref().display());
        let pool = SqlitePool::connect(&url).await?;
        let store = Self::new(pool);
        store.ensure_schema().await?;
        Ok(store)
    }

    pub async fn memory() -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        let store = Self::new(pool);
        store.ensure_schema().await?;
        Ok(store)
    }

    pub async fn ensure_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ruvo_tasks (
                id TEXT PRIMARY KEY,
                queue TEXT NOT NULL,
                payload BLOB NOT NULL,
                run_at TEXT NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                lease_until TEXT,
                dedup_key TEXT UNIQUE,
                status TEXT NOT NULL,
                worker TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS ruvo_tasks_claim_idx
            ON ruvo_tasks (queue, status, run_at)
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    fn next_id(&self) -> String {
        format!("t{}", self.seq.fetch_add(1, Ordering::SeqCst))
    }
}

fn to_dt(t: SystemTime) -> DateTime<Utc> {
    DateTime::<Utc>::from(t)
}

fn from_dt(dt: DateTime<Utc>) -> SystemTime {
    SystemTime::from(dt)
}

fn status_str(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Done => "done",
        TaskStatus::Failed => "failed",
    }
}

fn parse_status(s: &str) -> TaskStatus {
    match s {
        "running" => TaskStatus::Running,
        "done" => TaskStatus::Done,
        "failed" => TaskStatus::Failed,
        _ => TaskStatus::Pending,
    }
}

fn map_err(e: sqlx::Error) -> TaskError {
    if let sqlx::Error::Database(ref db) = e {
        if db.message().contains("UNIQUE") || db.message().contains("unique") {
            return TaskError::Duplicate;
        }
    }
    TaskError::Msg(e.to_string())
}

fn row_to_task(row: &sqlx::sqlite::SqliteRow) -> Result<Task, TaskError> {
    let status: String = row.try_get("status").map_err(map_err)?;
    let run_at_s: String = row.try_get("run_at").map_err(map_err)?;
    let run_at = DateTime::parse_from_rfc3339(&run_at_s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| TaskError::Msg(e.to_string()))?;
    let lease_until: Option<String> = row.try_get("lease_until").map_err(map_err)?;
    let lease_until = lease_until
        .map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| TaskError::Msg(e.to_string()))
        })
        .transpose()?;
    let payload: Vec<u8> = row.try_get("payload").map_err(map_err)?;
    Ok(Task {
        id: row.try_get("id").map_err(map_err)?,
        queue: row.try_get("queue").map_err(map_err)?,
        payload: Bytes::from(payload),
        run_at: from_dt(run_at),
        attempts: row.try_get::<i64, _>("attempts").map_err(map_err)? as u32,
        lease_until: lease_until.map(from_dt),
        dedup_key: row.try_get("dedup_key").map_err(map_err)?,
        status: parse_status(&status),
        worker: row.try_get("worker").map_err(map_err)?,
    })
}

impl TaskStore for SqliteTaskStore {
    fn enqueue<'a>(&'a self, opts: EnqueueOpts) -> BoxFuture<'a, Result<String, TaskError>> {
        Box::pin(async move {
            if let Some(ref dk) = opts.dedup_key {
                let existing: Option<(String,)> =
                    sqlx::query_as("SELECT id FROM ruvo_tasks WHERE dedup_key = ?")
                        .bind(dk)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(map_err)?;
                if let Some((id,)) = existing {
                    return Ok(id);
                }
            }

            let id = self.next_id();
            let run_at = to_dt(opts.run_at.unwrap_or_else(SystemTime::now)).to_rfc3339();
            let result = sqlx::query(
                r#"
                INSERT INTO ruvo_tasks
                    (id, queue, payload, run_at, attempts, lease_until, dedup_key, status, worker)
                VALUES (?, ?, ?, ?, 0, NULL, ?, ?, NULL)
                "#,
            )
            .bind(&id)
            .bind(&opts.queue)
            .bind(opts.payload.as_ref())
            .bind(&run_at)
            .bind(&opts.dedup_key)
            .bind(status_str(TaskStatus::Pending))
            .execute(&self.pool)
            .await;

            match result {
                Ok(_) => Ok(id),
                Err(e) => {
                    if let Some(ref dk) = opts.dedup_key {
                        if matches!(&e, sqlx::Error::Database(_)) {
                            let existing: Option<(String,)> =
                                sqlx::query_as("SELECT id FROM ruvo_tasks WHERE dedup_key = ?")
                                    .bind(dk)
                                    .fetch_optional(&self.pool)
                                    .await
                                    .map_err(map_err)?;
                            if let Some((eid,)) = existing {
                                return Ok(eid);
                            }
                        }
                    }
                    Err(map_err(e))
                }
            }
        })
    }

    fn claim<'a>(
        &'a self,
        queue: &'a str,
        worker: &'a str,
        lease: Duration,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Task>, TaskError>> {
        Box::pin(async move {
            let _guard = self.claim_lock.lock().await;
            let mut tx = self.pool.begin().await.map_err(map_err)?;

            let now = to_dt(SystemTime::now()).to_rfc3339();
            let lease_until = to_dt(SystemTime::now() + lease).to_rfc3339();
            let rows = sqlx::query(
                r#"
                SELECT id FROM ruvo_tasks
                WHERE queue = ?
                  AND status = 'pending'
                  AND run_at <= ?
                ORDER BY run_at
                LIMIT ?
                "#,
            )
            .bind(queue)
            .bind(&now)
            .bind(limit as i64)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_err)?;

            let mut claimed = Vec::new();
            for row in rows {
                let id: String = row.try_get("id").map_err(map_err)?;
                let updated = sqlx::query(
                    r#"
                    UPDATE ruvo_tasks
                    SET status = 'running',
                        attempts = attempts + 1,
                        worker = ?,
                        lease_until = ?
                    WHERE id = ? AND status = 'pending'
                    "#,
                )
                .bind(worker)
                .bind(&lease_until)
                .bind(&id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
                if updated.rows_affected() == 0 {
                    continue;
                }
                let full = sqlx::query(
                    r#"
                    SELECT id, queue, payload, run_at, attempts, lease_until, dedup_key, status, worker
                    FROM ruvo_tasks WHERE id = ?
                    "#,
                )
                .bind(&id)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_err)?;
                claimed.push(row_to_task(&full)?);
            }
            tx.commit().await.map_err(map_err)?;
            Ok(claimed)
        })
    }

    fn heartbeat<'a>(
        &'a self,
        id: &'a str,
        lease: Duration,
    ) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let lease_until = to_dt(SystemTime::now() + lease).to_rfc3339();
            let r = sqlx::query(
                r#"
                UPDATE ruvo_tasks
                SET lease_until = ?
                WHERE id = ? AND status = 'running'
                "#,
            )
            .bind(&lease_until)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            if r.rows_affected() == 0 {
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM ruvo_tasks WHERE id = ?")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(map_err)?;
                return match exists {
                    None => Err(TaskError::NotFound),
                    Some(_) => Err(TaskError::Msg("not running".into())),
                };
            }
            Ok(())
        })
    }

    fn complete<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let r = sqlx::query(
                r#"
                UPDATE ruvo_tasks
                SET status = 'done', lease_until = NULL, worker = NULL, dedup_key = NULL
                WHERE id = ?
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            if r.rows_affected() == 0 {
                return Err(TaskError::NotFound);
            }
            Ok(())
        })
    }

    fn fail<'a>(
        &'a self,
        id: &'a str,
        retry_at: Option<SystemTime>,
    ) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let r = if let Some(at) = retry_at {
                sqlx::query(
                    r#"
                    UPDATE ruvo_tasks
                    SET status = 'pending',
                        run_at = ?,
                        lease_until = NULL,
                        worker = NULL
                    WHERE id = ?
                    "#,
                )
                .bind(to_dt(at).to_rfc3339())
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
            } else {
                sqlx::query(
                    r#"
                    UPDATE ruvo_tasks
                    SET status = 'failed',
                        lease_until = NULL,
                        worker = NULL,
                        dedup_key = NULL
                    WHERE id = ?
                    "#,
                )
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
            };
            if r.rows_affected() == 0 {
                return Err(TaskError::NotFound);
            }
            Ok(())
        })
    }

    fn reap<'a>(&'a self, now: SystemTime) -> BoxFuture<'a, Result<u64, TaskError>> {
        Box::pin(async move {
            let r = sqlx::query(
                r#"
                UPDATE ruvo_tasks
                SET status = 'pending', lease_until = NULL, worker = NULL
                WHERE status = 'running'
                  AND lease_until IS NOT NULL
                  AND lease_until <= ?
                "#,
            )
            .bind(to_dt(now).to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            Ok(r.rows_affected())
        })
    }

    fn list<'a>(
        &'a self,
        queue: &'a str,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Task>, TaskError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT id, queue, payload, run_at, attempts, lease_until, dedup_key, status, worker
                FROM ruvo_tasks
                WHERE queue = ?
                ORDER BY id
                LIMIT ?
                "#,
            )
            .bind(queue)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
            rows.iter().map(row_to_task).collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn enqueue_claim_complete() {
        let store = SqliteTaskStore::memory().await.unwrap();
        let id = store
            .enqueue(EnqueueOpts {
                queue: "default".into(),
                payload: Bytes::from_static(b"{}"),
                run_at: None,
                dedup_key: None,
            })
            .await
            .unwrap();
        let claimed = store
            .claim("default", "w1", Duration::from_secs(30), 10)
            .await
            .unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].id, id);
        store.complete(&id).await.unwrap();
        let again = store
            .claim("default", "w1", Duration::from_secs(30), 10)
            .await
            .unwrap();
        assert!(again.is_empty());
    }
}
