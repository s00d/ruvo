//! Postgres [`TaskStore`](ruvo_tasks_store::TaskStore) with `FOR UPDATE SKIP LOCKED`.

use bytes::Bytes;
use chrono::{DateTime, Utc};
use ruvo_tasks_store::{
    BoxFuture, EnqueueOpts, Task, TaskError, TaskStatus, TaskStore,
};
use sqlx::{PgPool, Row};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Task queue on table `ruvo_tasks`.
#[derive(Clone)]
pub struct PostgresTaskStore {
    pool: PgPool,
    seq: Arc<AtomicU64>,
}

impl PostgresTaskStore {
    pub fn new(pool: PgPool) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Self {
            pool,
            seq: Arc::new(AtomicU64::new(seed)),
        }
    }

    /// Build from a Ruvo [`ruvo_db::DbPool`] (clones the underlying sqlx pool).
    pub async fn from_db_pool(pool: &ruvo_db::DbPool) -> Result<Self, ruvo_db::DbError> {
        Ok(Self::new(pool.postgres_pool().await?))
    }

    /// `CREATE TABLE IF NOT EXISTS ruvo_tasks(...)`.
    pub async fn ensure_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ruvo_tasks (
                id text PRIMARY KEY,
                queue text NOT NULL,
                payload bytea NOT NULL,
                run_at timestamptz NOT NULL,
                attempts integer NOT NULL DEFAULT 0,
                lease_until timestamptz,
                dedup_key text UNIQUE,
                status text NOT NULL,
                worker text
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
        if db.constraint().is_some_and(|c| c.contains("dedup")) {
            return TaskError::Duplicate;
        }
    }
    TaskError::Msg(e.to_string())
}

fn row_to_task(row: &sqlx::postgres::PgRow) -> Result<Task, TaskError> {
    let status: String = row.try_get("status").map_err(map_err)?;
    let run_at: DateTime<Utc> = row.try_get("run_at").map_err(map_err)?;
    let lease_until: Option<DateTime<Utc>> = row.try_get("lease_until").map_err(map_err)?;
    let payload: Vec<u8> = row.try_get("payload").map_err(map_err)?;
    Ok(Task {
        id: row.try_get("id").map_err(map_err)?,
        queue: row.try_get("queue").map_err(map_err)?,
        payload: Bytes::from(payload),
        run_at: from_dt(run_at),
        attempts: row.try_get::<i32, _>("attempts").map_err(map_err)? as u32,
        lease_until: lease_until.map(from_dt),
        dedup_key: row.try_get("dedup_key").map_err(map_err)?,
        status: parse_status(&status),
        worker: row.try_get("worker").map_err(map_err)?,
    })
}

impl TaskStore for PostgresTaskStore {
    fn enqueue<'a>(&'a self, opts: EnqueueOpts) -> BoxFuture<'a, Result<String, TaskError>> {
        Box::pin(async move {
            if let Some(ref dk) = opts.dedup_key {
                let existing: Option<(String,)> =
                    sqlx::query_as("SELECT id FROM ruvo_tasks WHERE dedup_key = $1")
                        .bind(dk)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(map_err)?;
                if let Some((id,)) = existing {
                    return Ok(id);
                }
            }

            let id = self.next_id();
            let run_at = to_dt(opts.run_at.unwrap_or_else(SystemTime::now));
            let result = sqlx::query(
                r#"
                INSERT INTO ruvo_tasks
                    (id, queue, payload, run_at, attempts, lease_until, dedup_key, status, worker)
                VALUES ($1, $2, $3, $4, 0, NULL, $5, $6, NULL)
                "#,
            )
            .bind(&id)
            .bind(&opts.queue)
            .bind(opts.payload.as_ref())
            .bind(run_at)
            .bind(&opts.dedup_key)
            .bind(status_str(TaskStatus::Pending))
            .execute(&self.pool)
            .await;

            match result {
                Ok(_) => Ok(id),
                Err(e) => {
                    // Race on dedup_key: return the winner's id.
                    if let Some(ref dk) = opts.dedup_key {
                        if let sqlx::Error::Database(ref db) = e {
                            if db.constraint().is_some_and(|c| c.contains("dedup")) {
                                let existing: Option<(String,)> = sqlx::query_as(
                                    "SELECT id FROM ruvo_tasks WHERE dedup_key = $1",
                                )
                                .bind(dk)
                                .fetch_optional(&self.pool)
                                .await
                                .map_err(map_err)?;
                                if let Some((eid,)) = existing {
                                    return Ok(eid);
                                }
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
            let lease_until = to_dt(SystemTime::now() + lease);
            let limit = limit as i64;
            let rows = sqlx::query(
                r#"
                WITH cte AS (
                    SELECT id FROM ruvo_tasks
                    WHERE queue = $1
                      AND status = 'pending'
                      AND run_at <= NOW()
                    ORDER BY run_at
                    LIMIT $2
                    FOR UPDATE SKIP LOCKED
                )
                UPDATE ruvo_tasks t
                SET status = 'running',
                    attempts = attempts + 1,
                    worker = $3,
                    lease_until = $4
                FROM cte
                WHERE t.id = cte.id
                RETURNING t.id, t.queue, t.payload, t.run_at, t.attempts,
                          t.lease_until, t.dedup_key, t.status, t.worker
                "#,
            )
            .bind(queue)
            .bind(limit)
            .bind(worker)
            .bind(lease_until)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

            rows.iter().map(row_to_task).collect()
        })
    }

    fn heartbeat<'a>(
        &'a self,
        id: &'a str,
        lease: Duration,
    ) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let lease_until = to_dt(SystemTime::now() + lease);
            let r = sqlx::query(
                r#"
                UPDATE ruvo_tasks
                SET lease_until = $2
                WHERE id = $1 AND status = 'running'
                "#,
            )
            .bind(id)
            .bind(lease_until)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            if r.rows_affected() == 0 {
                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT status FROM ruvo_tasks WHERE id = $1")
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
                WHERE id = $1
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
                        run_at = $2,
                        lease_until = NULL,
                        worker = NULL
                    WHERE id = $1
                    "#,
                )
                .bind(id)
                .bind(to_dt(at))
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
                    WHERE id = $1
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
                  AND lease_until <= $1
                "#,
            )
            .bind(to_dt(now))
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
                WHERE queue = $1
                ORDER BY id
                LIMIT $2
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
