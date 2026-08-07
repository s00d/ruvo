//! Maildir-style [`TaskStore`](crate::TaskStore) using `tokio::fs` + rename.

use bytes::Bytes;
use crate::{
    BoxFuture, EnqueueOpts, Task, TaskError, TaskStatus, TaskStore,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct TaskFile {
    id: String,
    queue: String,
    payload: Vec<u8>,
    run_at_ms: u64,
    attempts: u32,
    lease_until_ms: Option<u64>,
    dedup_key: Option<String>,
    status: String,
    worker: Option<String>,
}

fn to_ms(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn from_ms(ms: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms)
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

impl From<&Task> for TaskFile {
    fn from(t: &Task) -> Self {
        Self {
            id: t.id.clone(),
            queue: t.queue.clone(),
            payload: t.payload.to_vec(),
            run_at_ms: to_ms(t.run_at),
            attempts: t.attempts,
            lease_until_ms: t.lease_until.map(to_ms),
            dedup_key: t.dedup_key.clone(),
            status: status_str(t.status).into(),
            worker: t.worker.clone(),
        }
    }
}

impl From<TaskFile> for Task {
    fn from(f: TaskFile) -> Self {
        Self {
            id: f.id,
            queue: f.queue,
            payload: Bytes::from(f.payload),
            run_at: from_ms(f.run_at_ms),
            attempts: f.attempts,
            lease_until: f.lease_until_ms.map(from_ms),
            dedup_key: f.dedup_key,
            status: parse_status(&f.status),
            worker: f.worker,
        }
    }
}

async fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = tokio::fs::File::create(&tmp).await?;
        f.write_all(data).await?;
        f.flush().await?;
    }
    tokio::fs::rename(&tmp, path).await
}

#[derive(Clone)]
pub struct FileTaskStore {
    root: PathBuf,
    seq: Arc<AtomicU64>,
    /// Serialize claim/reap to avoid double-claim races.
    lock: Arc<Mutex<()>>,
}

impl FileTaskStore {
    pub async fn open(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        for sub in ["tmp", "pending", "active", "done", "failed", "meta"] {
            tokio::fs::create_dir_all(root.join(sub)).await?;
        }
        Ok(Self {
            root,
            seq: Arc::new(AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(1),
            )),
            lock: Arc::new(Mutex::new(())),
        })
    }

    fn path(&self, folder: &str, id: &str) -> PathBuf {
        self.root.join(folder).join(format!("{id}.json"))
    }

    async fn write_task(&self, folder: &str, task: &Task) -> Result<(), TaskError> {
        let path = self.path(folder, &task.id);
        let data = serde_json::to_vec(&TaskFile::from(task))
            .map_err(|e| TaskError::Msg(e.to_string()))?;
        atomic_write(&path, &data)
            .await
            .map_err(|e| TaskError::Msg(e.to_string()))
    }

    async fn read_task(&self, path: &Path) -> Result<Task, TaskError> {
        let data = tokio::fs::read(path)
            .await
            .map_err(|e| TaskError::Msg(e.to_string()))?;
        let f: TaskFile =
            serde_json::from_slice(&data).map_err(|e| TaskError::Msg(e.to_string()))?;
        Ok(f.into())
    }

    async fn find_id(&self, id: &str) -> Result<(PathBuf, Task), TaskError> {
        for folder in ["pending", "active", "done", "failed"] {
            let p = self.path(folder, id);
            if p.exists() {
                let t = self.read_task(&p).await?;
                return Ok((p, t));
            }
        }
        Err(TaskError::NotFound)
    }
}

impl TaskStore for FileTaskStore {
    fn enqueue<'a>(&'a self, opts: EnqueueOpts) -> BoxFuture<'a, Result<String, TaskError>> {
        Box::pin(async move {
            let _g = self.lock.lock().await;
            if let Some(ref dk) = opts.dedup_key {
                let meta = self.root.join("meta").join(format!("{dk}.id"));
                if meta.exists() {
                    let id = tokio::fs::read_to_string(&meta)
                        .await
                        .map_err(|e| TaskError::Msg(e.to_string()))?;
                    return Ok(id.trim().to_string());
                }
            }
            let id = format!("t{}", self.seq.fetch_add(1, Ordering::SeqCst));
            let task = Task {
                id: id.clone(),
                queue: opts.queue,
                payload: opts.payload,
                run_at: opts.run_at.unwrap_or_else(SystemTime::now),
                attempts: 0,
                lease_until: None,
                dedup_key: opts.dedup_key.clone(),
                status: TaskStatus::Pending,
                worker: None,
            };
            self.write_task("pending", &task).await?;
            if let Some(dk) = opts.dedup_key {
                let meta = self.root.join("meta").join(format!("{dk}.id"));
                atomic_write(&meta, id.as_bytes())
                    .await
                    .map_err(|e| TaskError::Msg(e.to_string()))?;
            }
            Ok(id)
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
            let _g = self.lock.lock().await;
            let now = SystemTime::now();
            let mut dir = tokio::fs::read_dir(self.root.join("pending"))
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            let mut candidates = Vec::new();
            while let Some(ent) = dir
                .next_entry()
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?
            {
                let path = ent.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let t = self.read_task(&path).await?;
                if t.queue == queue && t.run_at <= now {
                    candidates.push((t.run_at, path, t));
                }
            }
            candidates.sort_by_key(|(run_at, _, _)| *run_at);
            let mut out = Vec::new();
            for (_, path, mut t) in candidates.into_iter().take(limit) {
                t.status = TaskStatus::Running;
                t.attempts += 1;
                t.worker = Some(worker.to_string());
                t.lease_until = Some(now + lease);
                let dest = self.path("active", &t.id);
                self.write_task("active", &t).await?;
                let _ = tokio::fs::remove_file(&path).await;
                // ensure dest exists (write_task already wrote)
                let _ = dest;
                out.push(t);
            }
            Ok(out)
        })
    }

    fn heartbeat<'a>(
        &'a self,
        id: &'a str,
        lease: Duration,
    ) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let _g = self.lock.lock().await;
            let (path, mut t) = self.find_id(id).await?;
            if t.status != TaskStatus::Running {
                return Err(TaskError::Msg("not running".into()));
            }
            t.lease_until = Some(SystemTime::now() + lease);
            let data = serde_json::to_vec(&TaskFile::from(&t))
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            atomic_write(&path, &data)
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))
        })
    }

    fn complete<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let _g = self.lock.lock().await;
            let (path, mut t) = self.find_id(id).await?;
            t.status = TaskStatus::Done;
            t.lease_until = None;
            t.worker = None;
            self.write_task("done", &t).await?;
            let _ = tokio::fs::remove_file(&path).await;
            if let Some(dk) = t.dedup_key {
                let _ = tokio::fs::remove_file(self.root.join("meta").join(format!("{dk}.id"))).await;
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
            let _g = self.lock.lock().await;
            let (path, mut t) = self.find_id(id).await?;
            if let Some(at) = retry_at {
                t.status = TaskStatus::Pending;
                t.run_at = at;
                t.lease_until = None;
                t.worker = None;
                self.write_task("pending", &t).await?;
            } else {
                t.status = TaskStatus::Failed;
                t.lease_until = None;
                t.worker = None;
                self.write_task("failed", &t).await?;
                if let Some(dk) = &t.dedup_key {
                    let _ =
                        tokio::fs::remove_file(self.root.join("meta").join(format!("{dk}.id")))
                            .await;
                }
            }
            let _ = tokio::fs::remove_file(&path).await;
            Ok(())
        })
    }

    fn reap<'a>(&'a self, now: SystemTime) -> BoxFuture<'a, Result<u64, TaskError>> {
        Box::pin(async move {
            let _g = self.lock.lock().await;
            let mut dir = tokio::fs::read_dir(self.root.join("active"))
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            let mut n = 0u64;
            while let Some(ent) = dir
                .next_entry()
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?
            {
                let path = ent.path();
                let mut t = self.read_task(&path).await?;
                if let Some(until) = t.lease_until {
                    if until <= now {
                        t.status = TaskStatus::Pending;
                        t.lease_until = None;
                        t.worker = None;
                        self.write_task("pending", &t).await?;
                        let _ = tokio::fs::remove_file(&path).await;
                        n += 1;
                    }
                }
            }
            Ok(n)
        })
    }

    fn list<'a>(
        &'a self,
        queue: &'a str,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Task>, TaskError>> {
        Box::pin(async move {
            let mut out = Vec::new();
            for folder in ["pending", "active", "done", "failed"] {
                let mut dir = match tokio::fs::read_dir(self.root.join(folder)).await {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                while let Ok(Some(ent)) = dir.next_entry().await {
                    if out.len() >= limit {
                        break;
                    }
                    let path = ent.path();
                    if path.extension().and_then(|s| s.to_str()) != Some("json") {
                        continue;
                    }
                    if let Ok(t) = self.read_task(&path).await {
                        if t.queue == queue {
                            out.push(t);
                        }
                    }
                }
            }
            out.sort_by(|a, b| a.id.cmp(&b.id));
            out.truncate(limit);
            Ok(out)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn file_conformance() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(FileTaskStore::open(dir.path()).await.unwrap());
        crate::conformance::run(store).await;
    }
}
