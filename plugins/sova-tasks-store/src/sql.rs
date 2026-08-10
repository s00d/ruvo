//! SQL-backed [`TaskStore`] on the shared [`sova_db::DbPool`] (postgres / sqlite / mysql).

use crate::{BoxFuture, EnqueueOpts, Task, TaskError, TaskStatus, TaskStore};
use bytes::Bytes;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, QueryResult, Statement,
    TransactionTrait, Value,
};
use sova_db::DbPool;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// Task queue on table `sova_tasks`.
#[derive(Clone)]
pub struct SqlTaskStore {
    pool: DbPool,
    seq: Arc<AtomicU64>,
    /// Serializes claim for SQLite (no `SKIP LOCKED`).
    claim_lock: Arc<Mutex<()>>,
    schema_ready: Arc<AtomicBool>,
}

impl SqlTaskStore {
    pub fn new(pool: DbPool) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Self {
            pool,
            seq: Arc::new(AtomicU64::new(seed)),
            claim_lock: Arc::new(Mutex::new(())),
            schema_ready: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Bind to Sova [`DbPool`]. Schema is created on first use (pool may connect later).
    pub fn from_db_pool(pool: &DbPool) -> Self {
        Self::new(pool.clone())
    }

    pub async fn ensure_schema(&self) -> Result<(), DbErr> {
        let conn = self.pool.get().map_err(|e| e.0)?;
        self.ensure_schema_on(&conn).await?;
        self.schema_ready.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn ensure_schema_on(&self, conn: &DatabaseConnection) -> Result<(), DbErr> {
        let backend = conn.get_database_backend();
        let ddl = match backend {
            DbBackend::Postgres => {
                r#"
                CREATE TABLE IF NOT EXISTS sova_tasks (
                    id text PRIMARY KEY,
                    queue text NOT NULL,
                    payload bytea NOT NULL,
                    run_at text NOT NULL,
                    attempts integer NOT NULL DEFAULT 0,
                    lease_until text,
                    dedup_key text UNIQUE,
                    status text NOT NULL,
                    worker text,
                    priority integer NOT NULL DEFAULT 0
                )
                "#
            }
            DbBackend::MySql => {
                r#"
                CREATE TABLE IF NOT EXISTS sova_tasks (
                    id varchar(64) PRIMARY KEY,
                    queue varchar(255) NOT NULL,
                    payload longblob NOT NULL,
                    run_at varchar(40) NOT NULL,
                    attempts int NOT NULL DEFAULT 0,
                    lease_until varchar(40) NULL,
                    dedup_key varchar(255) NULL,
                    status varchar(32) NOT NULL,
                    worker varchar(255) NULL,
                    priority int NOT NULL DEFAULT 0,
                    UNIQUE KEY sova_tasks_dedup (dedup_key)
                )
                "#
            }
            _ => {
                r#"
                CREATE TABLE IF NOT EXISTS sova_tasks (
                    id TEXT PRIMARY KEY,
                    queue TEXT NOT NULL,
                    payload BLOB NOT NULL,
                    run_at TEXT NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    lease_until TEXT,
                    dedup_key TEXT UNIQUE,
                    status TEXT NOT NULL,
                    worker TEXT,
                    priority INTEGER NOT NULL DEFAULT 0
                )
                "#
            }
        };
        conn.execute_unprepared(ddl).await?;
        let claim_idx = match backend {
            DbBackend::MySql => {
                "CREATE INDEX sova_tasks_claim_idx ON sova_tasks (queue, status, priority, run_at)"
            }
            _ => {
                "CREATE INDEX IF NOT EXISTS sova_tasks_claim_idx ON sova_tasks (queue, status, priority, run_at)"
            }
        };
        let _ = conn.execute_unprepared(claim_idx).await;
        // `reap` filters running jobs by expired lease.
        let reap_idx = match backend {
            DbBackend::MySql => {
                "CREATE INDEX sova_tasks_reap_idx ON sova_tasks (status, lease_until)"
            }
            _ => {
                "CREATE INDEX IF NOT EXISTS sova_tasks_reap_idx ON sova_tasks (status, lease_until)"
            }
        };
        let _ = conn.execute_unprepared(reap_idx).await;
        Ok(())
    }

    async fn conn(&self) -> Result<DatabaseConnection, DbErr> {
        let conn = self.pool.get().map_err(|e| e.0)?;
        if !self.schema_ready.load(Ordering::Acquire) {
            self.ensure_schema_on(&conn).await?;
            self.schema_ready.store(true, Ordering::Release);
        }
        Ok(conn)
    }

    fn next_id(&self) -> String {
        format!("t{}", self.seq.fetch_add(1, Ordering::SeqCst))
    }
}

fn to_rfc(t: SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()
}

fn from_rfc(s: &str) -> Result<SystemTime, TaskError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| SystemTime::from(d.with_timezone(&chrono::Utc)))
        .map_err(|e| TaskError::Msg(e.to_string()))
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

fn map_err(e: DbErr) -> TaskError {
    let msg = e.to_string();
    if msg.contains("UNIQUE")
        || msg.contains("unique")
        || msg.contains("Duplicate")
        || msg.contains("dedup")
    {
        return TaskError::Duplicate;
    }
    TaskError::Msg(msg)
}

fn val_str(s: impl Into<String>) -> Value {
    Value::String(Some(s.into()))
}

fn val_str_opt(s: Option<&str>) -> Value {
    match s {
        Some(s) => Value::String(Some(s.to_string())),
        None => Value::String(None),
    }
}

fn val_bytes(b: &[u8]) -> Value {
    Value::Bytes(Some(b.to_vec()))
}

fn row_to_task(row: &QueryResult) -> Result<Task, TaskError> {
    let status: String = row.try_get_by("status").map_err(map_err)?;
    let run_at_s: String = row.try_get_by("run_at").map_err(map_err)?;
    let lease_until: Option<String> = row.try_get_by("lease_until").ok().flatten();
    let payload: Vec<u8> = row.try_get_by("payload").map_err(map_err)?;
    let attempts: i64 = row
        .try_get_by::<i64, _>("attempts")
        .or_else(|_| row.try_get_by::<i32, _>("attempts").map(i64::from))
        .unwrap_or(0);
    Ok(Task {
        id: row.try_get_by("id").map_err(map_err)?,
        queue: row.try_get_by("queue").map_err(map_err)?,
        payload: Bytes::from(payload),
        run_at: from_rfc(&run_at_s)?,
        attempts: attempts as u32,
        lease_until: lease_until.as_deref().map(from_rfc).transpose()?,
        dedup_key: row.try_get_by("dedup_key").ok().flatten(),
        status: parse_status(&status),
        worker: row.try_get_by("worker").ok().flatten(),
        priority: row
            .try_get_by::<i32, _>("priority")
            .or_else(|_| row.try_get_by::<i64, _>("priority").map(|v| v as i32))
            .unwrap_or(0),
    })
}

fn select_cols() -> &'static str {
    "id, queue, payload, run_at, attempts, lease_until, dedup_key, status, worker, priority"
}

impl TaskStore for SqlTaskStore {
    fn enqueue<'a>(&'a self, opts: EnqueueOpts) -> BoxFuture<'a, Result<String, TaskError>> {
        Box::pin(async move {
            let conn = self.conn().await.map_err(map_err)?;
            let backend = conn.get_database_backend();

            if let Some(ref dk) = opts.dedup_key {
                let stmt = Statement::from_sql_and_values(
                    backend,
                    "SELECT id FROM sova_tasks WHERE dedup_key = $1",
                    [val_str(dk.clone())],
                );
                if let Some(row) = conn.query_one_raw(stmt).await.map_err(map_err)? {
                    let id: String = row.try_get_by("id").map_err(map_err)?;
                    return Ok(id);
                }
            }

            let id = self.next_id();
            let run_at = to_rfc(opts.run_at.unwrap_or_else(SystemTime::now));
            let stmt = Statement::from_sql_and_values(
                backend,
                r#"
                INSERT INTO sova_tasks
                    (id, queue, payload, run_at, attempts, lease_until, dedup_key, status, worker, priority)
                VALUES ($1, $2, $3, $4, 0, NULL, $5, $6, NULL, $7)
                "#,
                [
                    val_str(id.clone()),
                    val_str(opts.queue),
                    val_bytes(opts.payload.as_ref()),
                    val_str(run_at),
                    val_str_opt(opts.dedup_key.as_deref()),
                    val_str(status_str(TaskStatus::Pending)),
                    Value::Int(Some(opts.priority)),
                ],
            );
            match conn.execute_raw(stmt).await {
                Ok(_) => Ok(id),
                Err(e) => {
                    if let Some(ref dk) = opts.dedup_key {
                        let stmt = Statement::from_sql_and_values(
                            backend,
                            "SELECT id FROM sova_tasks WHERE dedup_key = $1",
                            [val_str(dk.clone())],
                        );
                        if let Ok(Some(row)) = conn.query_one_raw(stmt).await {
                            let eid: String = row.try_get_by("id").map_err(map_err)?;
                            return Ok(eid);
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
            let conn = self.conn().await.map_err(map_err)?;
            let backend = conn.get_database_backend();
            let now = to_rfc(SystemTime::now());
            let lease_until = to_rfc(SystemTime::now() + lease);

            match backend {
                DbBackend::Sqlite => {
                    let _guard = self.claim_lock.lock().await;
                    claim_select_update(&conn, backend, queue, worker, &now, &lease_until, limit)
                        .await
                }
                _ => {
                    claim_skip_locked(&conn, backend, queue, worker, &now, &lease_until, limit)
                        .await
                }
            }
        })
    }

    fn heartbeat<'a>(
        &'a self,
        id: &'a str,
        lease: Duration,
    ) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let conn = self.conn().await.map_err(map_err)?;
            let backend = conn.get_database_backend();
            let lease_until = to_rfc(SystemTime::now() + lease);
            let r = conn
                .execute_raw(Statement::from_sql_and_values(
                    backend,
                    r#"
                    UPDATE sova_tasks
                    SET lease_until = $1
                    WHERE id = $2 AND status = 'running'
                    "#,
                    [val_str(lease_until), val_str(id)],
                ))
                .await
                .map_err(map_err)?;
            if r.rows_affected() == 0 {
                let exists = conn
                    .query_one_raw(Statement::from_sql_and_values(
                        backend,
                        "SELECT status FROM sova_tasks WHERE id = $1",
                        [val_str(id)],
                    ))
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
            let conn = self.conn().await.map_err(map_err)?;
            let backend = conn.get_database_backend();
            let r = conn
                .execute_raw(Statement::from_sql_and_values(
                    backend,
                    r#"
                    UPDATE sova_tasks
                    SET status = 'done', lease_until = NULL, worker = NULL, dedup_key = NULL
                    WHERE id = $1
                    "#,
                    [val_str(id)],
                ))
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
            let conn = self.conn().await.map_err(map_err)?;
            let backend = conn.get_database_backend();
            let r = if let Some(at) = retry_at {
                conn.execute_raw(Statement::from_sql_and_values(
                    backend,
                    r#"
                    UPDATE sova_tasks
                    SET status = 'pending',
                        run_at = $1,
                        lease_until = NULL,
                        worker = NULL
                    WHERE id = $2
                    "#,
                    [val_str(to_rfc(at)), val_str(id)],
                ))
                .await
                .map_err(map_err)?
            } else {
                conn.execute_raw(Statement::from_sql_and_values(
                    backend,
                    r#"
                    UPDATE sova_tasks
                    SET status = 'failed',
                        lease_until = NULL,
                        worker = NULL,
                        dedup_key = NULL
                    WHERE id = $1
                    "#,
                    [val_str(id)],
                ))
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
            let conn = self.conn().await.map_err(map_err)?;
            let backend = conn.get_database_backend();
            let r = conn
                .execute_raw(Statement::from_sql_and_values(
                    backend,
                    r#"
                    UPDATE sova_tasks
                    SET status = 'pending', lease_until = NULL, worker = NULL
                    WHERE status = 'running'
                      AND lease_until IS NOT NULL
                      AND lease_until <= $1
                    "#,
                    [val_str(to_rfc(now))],
                ))
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
            let conn = self.conn().await.map_err(map_err)?;
            let backend = conn.get_database_backend();
            let rows = conn
                .query_all_raw(Statement::from_sql_and_values(
                    backend,
                    format!(
                        r#"
                        SELECT {}
                        FROM sova_tasks
                        WHERE queue = $1
                        ORDER BY id
                        LIMIT $2
                        "#,
                        select_cols()
                    ),
                    [val_str(queue), Value::BigInt(Some(limit as i64))],
                ))
                .await
                .map_err(map_err)?;
            rows.iter().map(row_to_task).collect()
        })
    }
}

async fn claim_skip_locked(
    conn: &DatabaseConnection,
    backend: DbBackend,
    queue: &str,
    worker: &str,
    now: &str,
    lease_until: &str,
    limit: usize,
) -> Result<Vec<Task>, TaskError> {
    conn.transaction::<_, Vec<Task>, DbErr>(|txn| {
        let queue = queue.to_string();
        let worker = worker.to_string();
        let now = now.to_string();
        let lease_until = lease_until.to_string();
        Box::pin(async move {
            let rows = txn
                .query_all_raw(Statement::from_sql_and_values(
                    backend,
                    r#"
                    SELECT id FROM sova_tasks
                    WHERE queue = $1
                      AND status = 'pending'
                      AND run_at <= $2
                    ORDER BY priority DESC, run_at ASC, id ASC
                    LIMIT $3
                    FOR UPDATE SKIP LOCKED
                    "#,
                    [
                        val_str(queue),
                        val_str(now),
                        Value::BigInt(Some(limit as i64)),
                    ],
                ))
                .await?;

            let mut claimed = Vec::new();
            for row in rows {
                let id: String = row.try_get_by("id")?;
                let updated = txn
                    .execute_raw(Statement::from_sql_and_values(
                        backend,
                        r#"
                        UPDATE sova_tasks
                        SET status = 'running',
                            attempts = attempts + 1,
                            worker = $1,
                            lease_until = $2
                        WHERE id = $3 AND status = 'pending'
                        "#,
                        [
                            val_str(worker.clone()),
                            val_str(lease_until.clone()),
                            val_str(id.clone()),
                        ],
                    ))
                    .await?;
                if updated.rows_affected() == 0 {
                    continue;
                }
                let full = txn
                    .query_one_raw(Statement::from_sql_and_values(
                        backend,
                        format!("SELECT {} FROM sova_tasks WHERE id = $1", select_cols()),
                        [val_str(id)],
                    ))
                    .await?
                    .ok_or_else(|| DbErr::Custom("claimed row missing".into()))?;
                claimed.push(row_to_task(&full).map_err(|e| DbErr::Custom(e.to_string()))?);
            }
            Ok(claimed)
        })
    })
    .await
    .map_err(|e| map_err(DbErr::Custom(e.to_string())))
}

async fn claim_select_update(
    conn: &DatabaseConnection,
    backend: DbBackend,
    queue: &str,
    worker: &str,
    now: &str,
    lease_until: &str,
    limit: usize,
) -> Result<Vec<Task>, TaskError> {
    conn.transaction::<_, Vec<Task>, DbErr>(|txn| {
        let queue = queue.to_string();
        let worker = worker.to_string();
        let now = now.to_string();
        let lease_until = lease_until.to_string();
        Box::pin(async move {
            let rows = txn
                .query_all_raw(Statement::from_sql_and_values(
                    backend,
                    r#"
                    SELECT id FROM sova_tasks
                    WHERE queue = $1
                      AND status = 'pending'
                      AND run_at <= $2
                    ORDER BY priority DESC, run_at ASC, id ASC
                    LIMIT $3
                    "#,
                    [
                        val_str(queue),
                        val_str(now),
                        Value::BigInt(Some(limit as i64)),
                    ],
                ))
                .await?;

            let mut claimed = Vec::new();
            for row in rows {
                let id: String = row.try_get_by("id")?;
                let updated = txn
                    .execute_raw(Statement::from_sql_and_values(
                        backend,
                        r#"
                        UPDATE sova_tasks
                        SET status = 'running',
                            attempts = attempts + 1,
                            worker = $1,
                            lease_until = $2
                        WHERE id = $3 AND status = 'pending'
                        "#,
                        [
                            val_str(worker.clone()),
                            val_str(lease_until.clone()),
                            val_str(id.clone()),
                        ],
                    ))
                    .await?;
                if updated.rows_affected() == 0 {
                    continue;
                }
                let full = txn
                    .query_one_raw(Statement::from_sql_and_values(
                        backend,
                        format!("SELECT {} FROM sova_tasks WHERE id = $1", select_cols()),
                        [val_str(id)],
                    ))
                    .await?
                    .ok_or_else(|| DbErr::Custom("claimed row missing".into()))?;
                claimed.push(row_to_task(&full).map_err(|e| DbErr::Custom(e.to_string()))?);
            }
            Ok(claimed)
        })
    })
    .await
    .map_err(|e| map_err(DbErr::Custom(e.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    async fn mem_store() -> SqlTaskStore {
        let conn = Database::connect("sqlite::memory:").await.unwrap();
        let pool = DbPool::new();
        pool.set(conn);
        let store = SqlTaskStore::from_db_pool(&pool);
        store.ensure_schema().await.unwrap();
        store
    }

    #[tokio::test]
    async fn enqueue_claim_complete() {
        let store = mem_store().await;
        let id = store
            .enqueue(EnqueueOpts {
                queue: "default".into(),
                payload: Bytes::from_static(b"{}"),
                run_at: None,
                dedup_key: None,
                priority: 0,
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

    #[tokio::test]
    async fn dedup_returns_same_id() {
        let store = mem_store().await;
        let a = store
            .enqueue(EnqueueOpts {
                queue: "default".into(),
                payload: Bytes::from_static(b"1"),
                run_at: None,
                dedup_key: Some("k1".into()),
                priority: 0,
            })
            .await
            .unwrap();
        let b = store
            .enqueue(EnqueueOpts {
                queue: "default".into(),
                payload: Bytes::from_static(b"2"),
                run_at: None,
                dedup_key: Some("k1".into()),
                priority: 0,
            })
            .await
            .unwrap();
        assert_eq!(a, b);
        let claimed = store
            .claim("default", "w", Duration::from_secs(5), 10)
            .await
            .unwrap();
        assert_eq!(claimed.len(), 1);
    }

    #[tokio::test]
    async fn priority_claim_order() {
        let store = mem_store().await;
        let low = store
            .enqueue(EnqueueOpts {
                queue: "prio".into(),
                payload: Bytes::from_static(b"low"),
                run_at: None,
                dedup_key: None,
                priority: 1,
            })
            .await
            .unwrap();
        let high = store
            .enqueue(EnqueueOpts {
                queue: "prio".into(),
                payload: Bytes::from_static(b"high"),
                run_at: None,
                dedup_key: None,
                priority: 10,
            })
            .await
            .unwrap();
        let first = store
            .claim("prio", "w", Duration::from_secs(5), 1)
            .await
            .unwrap();
        assert_eq!(first[0].id, high);
        let second = store
            .claim("prio", "w", Duration::from_secs(5), 1)
            .await
            .unwrap();
        assert_eq!(second[0].id, low);
    }

    #[tokio::test]
    async fn heartbeat_fail_list() {
        let store = mem_store().await;
        let id = store
            .enqueue(EnqueueOpts {
                queue: "default".into(),
                payload: Bytes::from_static(b"x"),
                run_at: None,
                dedup_key: None,
                priority: 0,
            })
            .await
            .unwrap();
        let claimed = store
            .claim("default", "w1", Duration::from_secs(30), 1)
            .await
            .unwrap();
        assert_eq!(claimed[0].id, id);
        store.heartbeat(&id, Duration::from_secs(60)).await.unwrap();
        store.fail(&id, None).await.unwrap();
        let listed = store.list("default", 10).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, TaskStatus::Failed);
        let empty = store
            .claim("default", "w2", Duration::from_secs(5), 10)
            .await
            .unwrap();
        assert!(empty.is_empty());
    }
}
