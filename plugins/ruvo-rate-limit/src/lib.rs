//! Rate limiting for Ruvo (Express [`express-rate-limit`](https://www.npmjs.com/package/express-rate-limit)-style).

use ruvo_core::extend::named;
use ruvo_core::{with_state, App, ClientAddr, Plugin, Request, Response};
use ruvo_store::KvStore;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Result of a rate-limit check.
struct Outcome {
    allowed: bool,
    limit: u64,
    remaining: u64,
    /// Unix timestamp (seconds) when the window resets.
    reset: u64,
}

type KeyFn = Arc<dyn Fn(&Request) -> String + Send + Sync>;
type SkipFn = Arc<dyn Fn(&Request) -> bool + Send + Sync>;

/// Rate limiter plugin.
pub struct RateLimit {
    max: usize,
    window: Duration,
    max_entries: usize,
    message: String,
    key_fn: Option<KeyFn>,
    skip: Option<SkipFn>,
    backend: Backend,
}

enum Backend {
    LocalSliding,
    SharedFixed(Arc<dyn KvStore>),
}

impl RateLimit {
    /// Local process sliding window (default).
    pub fn new(max: usize, window: Duration) -> Self {
        Self {
            max,
            window,
            max_entries: 10_000,
            message: "Too Many Requests".into(),
            key_fn: None,
            skip: None,
            backend: Backend::LocalSliding,
        }
    }

    pub fn per_minute(max: usize) -> Self {
        Self::new(max, Duration::from_secs(60))
    }

    /// Cap distinct client keys retained for local sliding (default 10_000).
    pub fn max_entries(mut self, n: usize) -> Self {
        self.max_entries = n.max(1);
        self
    }

    /// 429 response body (default `"Too Many Requests"`).
    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.message = msg.into();
        self
    }

    /// Custom rate-limit key (default: client IP).
    pub fn key_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&Request) -> String + Send + Sync + 'static,
    {
        self.key_fn = Some(Arc::new(f));
        self
    }

    /// Skip limiting when this returns true.
    pub fn skip<F>(mut self, f: F) -> Self
    where
        F: Fn(&Request) -> bool + Send + Sync + 'static,
    {
        self.skip = Some(Arc::new(f));
        self
    }

    /// Multi-process / shared store: fixed window via atomic `incr`.
    pub fn fixed_window(store: Arc<dyn KvStore>, max: usize, window: Duration) -> Self {
        Self {
            max,
            window,
            max_entries: 10_000,
            message: "Too Many Requests".into(),
            key_fn: None,
            skip: None,
            backend: Backend::SharedFixed(store),
        }
    }
}

struct RateLimitRuntime {
    message: String,
    key_fn: Option<KeyFn>,
    skip: Option<SkipFn>,
    backend: RuntimeBackend,
}

enum RuntimeBackend {
    Sliding(Arc<SlidingWindow>),
    Fixed(Arc<FixedWindow>),
}

impl Plugin for RateLimit {
    fn id(&self) -> &'static str {
        "rate-limit"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Rate limit")
            .description("Per-key request rate limiting")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        let backend = match self.backend {
            Backend::LocalSliding => RuntimeBackend::Sliding(Arc::new(SlidingWindow::new(
                self.max,
                self.window,
                self.max_entries,
            ))),
            Backend::SharedFixed(store) => RuntimeBackend::Fixed(Arc::new(FixedWindow {
                store,
                max: self.max as u64,
                window: self.window,
            })),
        };
        let runtime = RateLimitRuntime {
            message: self.message,
            key_fn: self.key_fn,
            skip: self.skip,
            backend,
        };
        app.use_middleware(named(
            "rate-limit",
            with_state(runtime, |rt, req, next| async move {
                if rt.skip.as_ref().is_some_and(|f| f(&req)) {
                    return next(req).await;
                }
                let key = resolve_key(&req, rt.key_fn.as_ref());
                let out = match &rt.backend {
                    RuntimeBackend::Sliding(w) => w.check(&key),
                    RuntimeBackend::Fixed(w) => w.check(&key).await,
                };
                if !out.allowed {
                    return limited(rt.message.as_str(), &out);
                }
                let mut res = next(req).await;
                attach_headers(&mut res, &out);
                res
            }),
        ));
    }
}

fn resolve_key(req: &Request, key_fn: Option<&KeyFn>) -> String {
    if let Some(f) = key_fn {
        return f(req);
    }
    client_ip(req).to_string()
}

fn client_ip(req: &Request) -> IpAddr {
    req.get::<ClientAddr>()
        .map(|a| a.0.ip())
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]))
}

fn limited(message: &str, out: &Outcome) -> Response {
    let mut res = Response::text(message).status(429);
    attach_headers(&mut res, out);
    res
}

fn attach_headers(res: &mut Response, out: &Outcome) {
    if let Ok(v) = out.limit.to_string().parse() {
        res.headers_mut().insert("ratelimit-limit", v);
    }
    if let Ok(v) = out.remaining.to_string().parse() {
        res.headers_mut().insert("ratelimit-remaining", v);
    }
    if let Ok(v) = out.reset.to_string().parse() {
        res.headers_mut().insert("ratelimit-reset", v);
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

struct SlidingWindow {
    inner: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max: usize,
    window: Duration,
    max_entries: usize,
}

impl SlidingWindow {
    fn new(max: usize, window: Duration, max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max,
            window,
            max_entries,
        }
    }

    fn check(&self, key: &str) -> Outcome {
        let mut map = self.inner.lock().unwrap();
        let now = Instant::now();
        let reset = unix_now() + self.window.as_secs().max(1);

        if let Some(entries) = map.get_mut(key) {
            entries.retain(|t| now.duration_since(*t) < self.window);
            if entries.is_empty() {
                map.remove(key);
            }
        }

        if map.len() >= self.max_entries && !map.contains_key(key) {
            map.retain(|_, v| {
                v.retain(|t| now.duration_since(*t) < self.window);
                !v.is_empty()
            });
            while map.len() >= self.max_entries {
                if let Some(k) = map.keys().next().cloned() {
                    map.remove(&k);
                } else {
                    break;
                }
            }
        }

        let entries = map.entry(key.to_string()).or_default();
        entries.retain(|t| now.duration_since(*t) < self.window);
        let limit = self.max as u64;
        if entries.len() >= self.max {
            return Outcome {
                allowed: false,
                limit,
                remaining: 0,
                reset,
            };
        }
        entries.push(now);
        let remaining = limit.saturating_sub(entries.len() as u64);
        Outcome {
            allowed: true,
            limit,
            remaining,
            reset,
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    #[cfg(test)]
    fn allow(&self, key: &str) -> bool {
        self.check(key).allowed
    }
}

struct FixedWindow {
    store: Arc<dyn KvStore>,
    max: u64,
    window: Duration,
}

impl FixedWindow {
    async fn check(&self, key: &str) -> Outcome {
        let secs = self.window.as_secs().max(1);
        let now = unix_now();
        let bucket = now / secs;
        let reset = (bucket + 1) * secs;
        let store_key = format!("rl:{key}:{bucket}");
        let n = self.store.incr(&store_key, 1, Some(self.window)).await;
        let limit = self.max;
        if n > self.max {
            Outcome {
                allowed: false,
                limit,
                remaining: 0,
                reset,
            }
        } else {
            Outcome {
                allowed: true,
                limit,
                remaining: limit.saturating_sub(n),
                reset,
            }
        }
    }

    #[cfg(test)]
    async fn allow(&self, key: &str) -> bool {
        self.check(key).await.allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruvo_store::MemoryStore;

    #[test]
    fn max_entries_caps_distinct_keys() {
        let w = SlidingWindow::new(100, Duration::from_secs(60), 3);
        assert!(w.allow("1"));
        assert!(w.allow("2"));
        assert!(w.allow("3"));
        assert_eq!(w.len(), 3);
        assert!(w.allow("4"));
        assert!(w.len() <= 3);
    }

    #[test]
    fn rate_blocks_after_max() {
        let w = SlidingWindow::new(2, Duration::from_secs(60), 10);
        assert!(w.allow("ip"));
        assert!(w.allow("ip"));
        assert!(!w.allow("ip"));
    }

    #[tokio::test]
    async fn fixed_window_incr() {
        let store = Arc::new(MemoryStore::new());
        let fw = FixedWindow {
            store,
            max: 2,
            window: Duration::from_secs(60),
        };
        assert!(fw.allow("ip").await);
        assert!(fw.allow("ip").await);
        assert!(!fw.allow("ip").await);
    }
}
