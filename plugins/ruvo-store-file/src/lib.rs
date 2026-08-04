//! File-backed [`KvStore`](ruvo_store::KvStore): HashMap in RAM + append-only log + snapshot.
//!
//! Reads never hit disk. Survives process restart without an external DB.

use bytes::Bytes;
use ruvo_store::{BoxFuture, KvStore};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Durability {
    /// Flush buffers; no fsync.
    Relaxed,
    /// `sync_all` after each append.
    Fsync,
}

struct Entry {
    val: Bytes,
    exp: Option<Instant>,
}

enum Op {
    Set { key: String, val: Bytes, ttl_ms: Option<u64> },
    Remove { key: String },
    ClearPrefix { prefix: String },
}

struct Inner {
    map: HashMap<String, Entry>,
    log: tokio::fs::File,
    ops_since_snap: u64,
    dir: PathBuf,
    durability: Durability,
}

#[derive(Clone)]
pub struct FileStore {
    inner: Arc<Mutex<Inner>>,
}

impl FileStore {
    pub async fn open(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        Self::open_with(dir, Durability::Relaxed).await
    }

    pub async fn open_with(dir: impl AsRef<Path>, durability: Durability) -> std::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        tokio::fs::create_dir_all(&dir).await?;
        let snap = dir.join("snapshot.bin");
        let log_path = dir.join("append.log");
        let mut map = load_snapshot(&snap).await.unwrap_or_default();
        if log_path.exists() {
            replay_log(&log_path, &mut map).await?;
        }
        let log = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await?;
        Ok(Self {
            inner: Arc::new(Mutex::new(Inner {
                map,
                log,
                ops_since_snap: 0,
                dir,
                durability,
            })),
        })
    }

    pub fn durability(self, d: Durability) -> Self {
        // Apply on next lock — store in Arc by reopening pattern; mutate now:
        let inner = self.inner.clone();
        tokio::spawn(async move {
            inner.lock().await.durability = d;
        });
        self
    }
}

fn alive(e: &Entry, now: Instant) -> bool {
    e.exp.map(|t| t > now).unwrap_or(true)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

async fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = tokio::fs::File::create(&tmp).await?;
        f.write_all(data).await?;
        f.flush().await?;
        f.sync_all().await?;
    }
    tokio::fs::rename(&tmp, path).await
}

async fn load_snapshot(path: &Path) -> std::io::Result<HashMap<String, Entry>> {
    let data = tokio::fs::read(path).await?;
    let mut map = HashMap::new();
    let mut i = 0;
    while i + 8 <= data.len() {
        let klen = u32::from_le_bytes(data[i..i + 4].try_into().unwrap()) as usize;
        let vlen = u32::from_le_bytes(data[i + 4..i + 8].try_into().unwrap()) as usize;
        i += 8;
        if i + klen + vlen + 8 > data.len() {
            break;
        }
        let key = String::from_utf8_lossy(&data[i..i + klen]).into_owned();
        i += klen;
        let val = Bytes::copy_from_slice(&data[i..i + vlen]);
        i += vlen;
        let exp_ms = u64::from_le_bytes(data[i..i + 8].try_into().unwrap());
        i += 8;
        let exp = if exp_ms == 0 {
            None
        } else {
            let now = now_ms();
            if exp_ms <= now {
                continue;
            }
            Some(Instant::now() + Duration::from_millis(exp_ms - now))
        };
        map.insert(key, Entry { val, exp });
    }
    Ok(map)
}

async fn write_snapshot(dir: &Path, map: &HashMap<String, Entry>) -> std::io::Result<()> {
    let mut buf = Vec::new();
    let now_i = Instant::now();
    let now = now_ms();
    for (k, e) in map {
        if !alive(e, now_i) {
            continue;
        }
        let exp_ms = e
            .exp
            .map(|t| now + t.saturating_duration_since(now_i).as_millis() as u64)
            .unwrap_or(0);
        buf.extend_from_slice(&(k.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(e.val.len() as u32).to_le_bytes());
        buf.extend_from_slice(k.as_bytes());
        buf.extend_from_slice(&e.val);
        buf.extend_from_slice(&exp_ms.to_le_bytes());
    }
    atomic_write(&dir.join("snapshot.bin"), &buf).await?;
    // truncate log
    atomic_write(&dir.join("append.log"), b"").await?;
    Ok(())
}

async fn append_op(inner: &mut Inner, op: &Op) -> std::io::Result<()> {
    let line = match op {
        Op::Set { key, val, ttl_ms } => {
            let mut v = Vec::new();
            v.push(b'S');
            v.extend_from_slice(&(key.len() as u32).to_le_bytes());
            v.extend_from_slice(&(val.len() as u32).to_le_bytes());
            v.extend_from_slice(&ttl_ms.unwrap_or(0).to_le_bytes());
            v.extend_from_slice(key.as_bytes());
            v.extend_from_slice(val);
            v
        }
        Op::Remove { key } => {
            let mut v = vec![b'R'];
            v.extend_from_slice(&(key.len() as u32).to_le_bytes());
            v.extend_from_slice(key.as_bytes());
            v
        }
        Op::ClearPrefix { prefix } => {
            let mut v = vec![b'C'];
            v.extend_from_slice(&(prefix.len() as u32).to_le_bytes());
            v.extend_from_slice(prefix.as_bytes());
            v
        }
    };
    inner.log.write_all(&(line.len() as u32).to_le_bytes()).await?;
    inner.log.write_all(&line).await?;
    inner.log.flush().await?;
    if inner.durability == Durability::Fsync {
        inner.log.sync_all().await?;
    }
    inner.ops_since_snap += 1;
    if inner.ops_since_snap >= 256 {
        write_snapshot(&inner.dir, &inner.map).await?;
        inner.log = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(inner.dir.join("append.log"))
            .await?;
        inner.ops_since_snap = 0;
    }
    Ok(())
}

async fn replay_log(path: &Path, map: &mut HashMap<String, Entry>) -> std::io::Result<()> {
    let data = tokio::fs::read(path).await?;
    let mut i = 0;
    while i + 4 <= data.len() {
        let n = u32::from_le_bytes(data[i..i + 4].try_into().unwrap()) as usize;
        i += 4;
        if i + n > data.len() {
            break;
        }
        let line = &data[i..i + n];
        i += n;
        if line.is_empty() {
            continue;
        }
        match line[0] {
            b'S' if line.len() >= 1 + 4 + 4 + 8 => {
                let klen = u32::from_le_bytes(line[1..5].try_into().unwrap()) as usize;
                let vlen = u32::from_le_bytes(line[5..9].try_into().unwrap()) as usize;
                let ttl_ms = u64::from_le_bytes(line[9..17].try_into().unwrap());
                let base = 17;
                if base + klen + vlen > line.len() {
                    continue;
                }
                let key = String::from_utf8_lossy(&line[base..base + klen]).into_owned();
                let val = Bytes::copy_from_slice(&line[base + klen..base + klen + vlen]);
                let exp = if ttl_ms == 0 {
                    None
                } else {
                    Some(Instant::now() + Duration::from_millis(ttl_ms))
                };
                map.insert(key, Entry { val, exp });
            }
            b'R' if line.len() >= 5 => {
                let klen = u32::from_le_bytes(line[1..5].try_into().unwrap()) as usize;
                if 5 + klen <= line.len() {
                    let key = String::from_utf8_lossy(&line[5..5 + klen]);
                    map.remove(key.as_ref());
                }
            }
            b'C' if line.len() >= 5 => {
                let plen = u32::from_le_bytes(line[1..5].try_into().unwrap()) as usize;
                if 5 + plen <= line.len() {
                    let prefix = String::from_utf8_lossy(&line[5..5 + plen]);
                    map.retain(|k, _| !k.starts_with(prefix.as_ref()));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

impl KvStore for FileStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            let now = Instant::now();
            match g.map.get(key) {
                Some(e) if alive(e, now) => Some(e.val.clone()),
                Some(_) => {
                    g.map.remove(key);
                    None
                }
                None => None,
            }
        })
    }

    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            let ttl_ms = ttl.map(|d| d.as_millis() as u64);
            g.map.insert(
                key.to_string(),
                Entry {
                    val: val.clone(),
                    exp: ttl.map(|d| Instant::now() + d),
                },
            );
            let _ = append_op(
                &mut g,
                &Op::Set {
                    key: key.to_string(),
                    val,
                    ttl_ms,
                },
            )
            .await;
        })
    }

    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            g.map.remove(key);
            let _ = append_op(&mut g, &Op::Remove { key: key.to_string() }).await;
        })
    }

    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            let now = Instant::now();
            let cur = match g.map.get(key) {
                Some(e) if alive(e, now) => {
                    std::str::from_utf8(&e.val).unwrap_or("0").parse().unwrap_or(0)
                }
                _ => 0,
            };
            let next = (cur + by).max(0) as u64;
            let val = Bytes::from(next.to_string());
            let ttl_ms = ttl.map(|d| d.as_millis() as u64);
            g.map.insert(
                key.to_string(),
                Entry {
                    val: val.clone(),
                    exp: ttl.map(|d| now + d),
                },
            );
            let _ = append_op(
                &mut g,
                &Op::Set {
                    key: key.to_string(),
                    val,
                    ttl_ms,
                },
            )
            .await;
            next
        })
    }

    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64> {
        Box::pin(async move {
            let mut g = self.inner.lock().await;
            let keys: Vec<_> = g
                .map
                .keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect();
            let n = keys.len() as u64;
            for k in keys {
                g.map.remove(&k);
            }
            let _ = append_op(
                &mut g,
                &Op::ClearPrefix {
                    prefix: prefix.to_string(),
                },
            )
            .await;
            n
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn file_conformance_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(FileStore::open(dir.path()).await.unwrap());
        ruvo_store::conformance::run(store.clone()).await;

        store
            .set("persist", Bytes::from_static(b"yes"), None)
            .await;
        drop(store);
        let store2 = FileStore::open(dir.path()).await.unwrap();
        assert_eq!(
            store2.get("persist").await.as_deref(),
            Some(b"yes".as_slice())
        );
    }
}
