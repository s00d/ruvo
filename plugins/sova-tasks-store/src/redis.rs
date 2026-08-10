//! Redis-backed [`TaskStore`] on the shared [`sova_redis::RedisPool`].

use crate::{BoxFuture, EnqueueOpts, Task, TaskError, TaskStatus, TaskStore};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use bytes::Bytes;
use redis::AsyncCommands;
use sova_redis::RedisPool;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SEQ_KEY: &str = "sova:t:seq";
const LEASE_ZSET: &str = "sova:t:lease";

fn ready_key(queue: &str) -> String {
    format!("sova:t:{queue}:ready")
}

fn all_key(queue: &str) -> String {
    format!("sova:t:{queue}:all")
}

fn hash_key(id: &str) -> String {
    format!("sova:t:h:{id}")
}

fn dedup_key(key: &str) -> String {
    format!("sova:t:dedup:{key}")
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn ms_from_time(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn time_from_ms(ms: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms.max(0) as u64)
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

/// Task queue on Redis / Valkey with lease semantics.
#[derive(Clone)]
pub struct RedisTaskStore {
    pool: RedisPool,
}

impl RedisTaskStore {
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    pub fn from_redis_pool(pool: &RedisPool) -> Self {
        Self::new(pool.clone())
    }

    async fn load_task(
        conn: &mut redis::aio::ConnectionManager,
        id: &str,
    ) -> Result<Option<Task>, TaskError> {
        let key = hash_key(id);
        let map: std::collections::HashMap<String, String> = conn
            .hgetall(&key)
            .await
            .map_err(|e| TaskError::Msg(e.to_string()))?;
        if map.is_empty() {
            return Ok(None);
        }
        Ok(Some(task_from_map(id, &map)?))
    }
}

fn task_from_map(
    id: &str,
    map: &std::collections::HashMap<String, String>,
) -> Result<Task, TaskError> {
    let payload_b64 = map
        .get("payload")
        .ok_or_else(|| TaskError::Msg("missing payload".into()))?;
    let payload = B64
        .decode(payload_b64)
        .map_err(|e| TaskError::Msg(e.to_string()))?;
    let run_at = map
        .get("run_at")
        .and_then(|s| s.parse::<i64>().ok())
        .map(time_from_ms)
        .unwrap_or_else(SystemTime::now);
    let attempts = map
        .get("attempts")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let lease_until = map
        .get("lease_until")
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i64>().ok())
        .map(time_from_ms);
    let status = map
        .get("status")
        .map(|s| parse_status(s))
        .unwrap_or(TaskStatus::Pending);
    let worker = map.get("worker").filter(|s| !s.is_empty()).cloned();
    let dedup = map.get("dedup_key").filter(|s| !s.is_empty()).cloned();
    let queue = map
        .get("queue")
        .cloned()
        .unwrap_or_else(|| "default".into());
    let priority = map
        .get("priority")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    Ok(Task {
        id: id.to_string(),
        queue,
        payload: Bytes::from(payload),
        run_at,
        attempts,
        lease_until,
        dedup_key: dedup,
        status,
        worker,
        priority,
    })
}

// Claim due tasks, preferring higher priority then earlier run_at.
const CLAIM_LUA: &str = r#"
local ready = KEYS[1]
local lease = KEYS[2]
local now = tonumber(ARGV[1])
local lease_ms = tonumber(ARGV[2])
local limit = tonumber(ARGV[3])
local worker = ARGV[4]
local prefix = ARGV[5]
local cand_limit = math.max(limit * 50, 100)
local candidates = redis.call('ZRANGEBYSCORE', ready, '-inf', now, 'LIMIT', 0, cand_limit)
local items = {}
for _, id in ipairs(candidates) do
  local hk = prefix .. id
  local p = tonumber(redis.call('HGET', hk, 'priority') or '0') or 0
  local r = tonumber(redis.call('HGET', hk, 'run_at') or '0') or 0
  table.insert(items, {id = id, p = p, r = r})
end
table.sort(items, function(a, b)
  if a.p ~= b.p then return a.p > b.p end
  if a.r ~= b.r then return a.r < b.r end
  return a.id < b.id
end)
local out = {}
for i = 1, math.min(limit, #items) do
  local id = items[i].id
  redis.call('ZREM', ready, id)
  local until_ms = now + lease_ms
  redis.call('ZADD', lease, until_ms, id)
  local hk = prefix .. id
  redis.call('HSET', hk, 'status', 'running', 'worker', worker, 'lease_until', tostring(until_ms))
  redis.call('HINCRBY', hk, 'attempts', 1)
  table.insert(out, id)
end
return out
"#;

const REAP_LUA: &str = r#"
local lease = KEYS[1]
local now = tonumber(ARGV[1])
local prefix = ARGV[2]
local ids = redis.call('ZRANGEBYSCORE', lease, '-inf', now)
local n = 0
for _, id in ipairs(ids) do
  local hk = prefix .. id
  local queue = redis.call('HGET', hk, 'queue')
  local run_at = redis.call('HGET', hk, 'run_at')
  if queue then
    redis.call('HSET', hk, 'status', 'pending', 'worker', '', 'lease_until', '')
    redis.call('ZREM', lease, id)
    local score = tonumber(run_at) or now
    redis.call('ZADD', 'sova:t:' .. queue .. ':ready', score, id)
    n = n + 1
  else
    redis.call('ZREM', lease, id)
  end
end
return n
"#;

impl TaskStore for RedisTaskStore {
    fn enqueue<'a>(&'a self, opts: EnqueueOpts) -> BoxFuture<'a, Result<String, TaskError>> {
        Box::pin(async move {
            let mut conn = self.pool.get().map_err(|e| TaskError::Msg(e.to_string()))?;

            if let Some(ref dk) = opts.dedup_key {
                let existing: Option<String> = conn
                    .get(dedup_key(dk))
                    .await
                    .map_err(|e| TaskError::Msg(e.to_string()))?;
                if let Some(id) = existing {
                    return Ok(id);
                }
            }

            let seq: i64 = conn
                .incr(SEQ_KEY, 1i64)
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            let id = format!("t{seq}");
            let run_at = opts.run_at.unwrap_or_else(SystemTime::now);
            let run_ms = ms_from_time(run_at);
            let payload = B64.encode(opts.payload.as_ref());
            let hk = hash_key(&id);
            let priority_s = opts.priority.to_string();

            let mut pipe = redis::pipe();
            pipe.atomic();
            pipe.hset_multiple(
                &hk,
                &[
                    ("queue", opts.queue.as_str()),
                    ("payload", payload.as_str()),
                    ("run_at", &run_ms.to_string()),
                    ("attempts", "0"),
                    ("lease_until", ""),
                    ("status", status_str(TaskStatus::Pending)),
                    ("worker", ""),
                    ("dedup_key", opts.dedup_key.as_deref().unwrap_or("")),
                    ("priority", priority_s.as_str()),
                ],
            );
            pipe.zadd(ready_key(&opts.queue), &id, run_ms);
            pipe.sadd(all_key(&opts.queue), &id);
            if let Some(ref dk) = opts.dedup_key {
                pipe.set(dedup_key(dk), &id);
            }
            pipe.query_async::<()>(&mut conn)
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
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
            let mut conn = self.pool.get().map_err(|e| TaskError::Msg(e.to_string()))?;
            let ids: Vec<String> = redis::Script::new(CLAIM_LUA)
                .key(ready_key(queue))
                .key(LEASE_ZSET)
                .arg(now_ms())
                .arg(lease.as_millis() as i64)
                .arg(limit as i64)
                .arg(worker)
                .arg("sova:t:h:")
                .invoke_async(&mut conn)
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;

            let mut out = Vec::with_capacity(ids.len());
            for id in &ids {
                if let Some(t) = Self::load_task(&mut conn, id).await? {
                    out.push(t);
                }
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
            let mut conn = self.pool.get().map_err(|e| TaskError::Msg(e.to_string()))?;
            let hk = hash_key(id);
            let status: Option<String> = conn
                .hget(&hk, "status")
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            match status.as_deref() {
                None => return Err(TaskError::NotFound),
                Some("running") => {}
                Some(_) => return Err(TaskError::Msg("not running".into())),
            }
            let until = now_ms() + lease.as_millis() as i64;
            let _: () = conn
                .hset(&hk, "lease_until", until.to_string())
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            let _: () = conn
                .zadd(LEASE_ZSET, id, until)
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            Ok(())
        })
    }

    fn complete<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let mut conn = self.pool.get().map_err(|e| TaskError::Msg(e.to_string()))?;
            let Some(task) = Self::load_task(&mut conn, id).await? else {
                return Err(TaskError::NotFound);
            };
            let hk = hash_key(id);
            let mut pipe = redis::pipe();
            pipe.atomic();
            pipe.hset_multiple(
                &hk,
                &[
                    ("status", status_str(TaskStatus::Done)),
                    ("worker", ""),
                    ("lease_until", ""),
                ],
            );
            pipe.zrem(LEASE_ZSET, id);
            pipe.zrem(ready_key(&task.queue), id);
            if let Some(ref dk) = task.dedup_key {
                pipe.del(dedup_key(dk));
            }
            pipe.query_async::<()>(&mut conn)
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            Ok(())
        })
    }

    fn fail<'a>(
        &'a self,
        id: &'a str,
        retry_at: Option<SystemTime>,
    ) -> BoxFuture<'a, Result<(), TaskError>> {
        Box::pin(async move {
            let mut conn = self.pool.get().map_err(|e| TaskError::Msg(e.to_string()))?;
            let Some(task) = Self::load_task(&mut conn, id).await? else {
                return Err(TaskError::NotFound);
            };
            let hk = hash_key(id);
            let mut pipe = redis::pipe();
            pipe.atomic();
            pipe.zrem(LEASE_ZSET, id);
            if let Some(at) = retry_at {
                let run_ms = ms_from_time(at);
                let run_ms_s = run_ms.to_string();
                pipe.hset_multiple(
                    &hk,
                    &[
                        ("status", status_str(TaskStatus::Pending)),
                        ("worker", ""),
                        ("lease_until", ""),
                        ("run_at", run_ms_s.as_str()),
                    ],
                );
                pipe.zadd(ready_key(&task.queue), id, run_ms);
            } else {
                pipe.hset_multiple(
                    &hk,
                    &[
                        ("status", status_str(TaskStatus::Failed)),
                        ("worker", ""),
                        ("lease_until", ""),
                    ],
                );
                pipe.zrem(ready_key(&task.queue), id);
                if let Some(ref dk) = task.dedup_key {
                    pipe.del(dedup_key(dk));
                }
            }
            pipe.query_async::<()>(&mut conn)
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            Ok(())
        })
    }

    fn reap<'a>(&'a self, now: SystemTime) -> BoxFuture<'a, Result<u64, TaskError>> {
        Box::pin(async move {
            let mut conn = self.pool.get().map_err(|e| TaskError::Msg(e.to_string()))?;
            let n: u64 = redis::Script::new(REAP_LUA)
                .key(LEASE_ZSET)
                .arg(ms_from_time(now))
                .arg("sova:t:h:")
                .invoke_async(&mut conn)
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            Ok(n)
        })
    }

    fn list<'a>(
        &'a self,
        queue: &'a str,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Task>, TaskError>> {
        Box::pin(async move {
            let mut conn = self.pool.get().map_err(|e| TaskError::Msg(e.to_string()))?;
            let mut ids: Vec<String> = conn
                .smembers(all_key(queue))
                .await
                .map_err(|e| TaskError::Msg(e.to_string()))?;
            ids.sort();
            ids.truncate(limit);
            let mut out = Vec::new();
            for id in ids {
                if let Some(t) = Self::load_task(&mut conn, &id).await? {
                    out.push(t);
                }
            }
            Ok(out)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conformance;
    use std::sync::Arc;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn redis_conformance() {
        let url = match std::env::var("REDIS_URL") {
            Ok(u) if !u.is_empty() => u,
            _ => {
                eprintln!("skip redis_conformance: REDIS_URL not set");
                return;
            }
        };
        let conn = match RedisPool::connect(&url).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("skip redis_conformance: connect failed: {e}");
                return;
            }
        };
        let pool = RedisPool::new();
        pool.set(conn);
        // Isolate from other DBs / runs.
        let store = RedisTaskStore::from_redis_pool(&pool);
        // Flush only our keys via a unique queue namespace is hard; use FLUSHDB only on test DB.
        let mut c = pool.get().unwrap();
        let _: () = redis::cmd("FLUSHDB").query_async(&mut c).await.unwrap();
        conformance::run(Arc::new(store) as Arc<dyn TaskStore>).await;
    }
}
