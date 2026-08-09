//! Rate limiting for Sova (Express [`express-rate-limit`](https://www.npmjs.com/package/express-rate-limit)-style).

mod events;

pub use events::RateLimitExceeded;

use sova_core::extend::{named, MwEntry};
use sova_core::{
    with_state, App, ClientAddr, EventBus, Plugin, RateLimitIdentity, Request, Response,
};
use sova_store::KvStore;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How the rate-limit key is derived (overridden by [`RateLimit::key_fn`]).
#[derive(Clone, Copy, Debug, Default)]
pub enum RateLimitKey {
    /// Client IP ([`ClientAddr`]).
    #[default]
    Ip,
    /// [`RateLimitIdentity`] when set, otherwise IP.
    Identity,
    /// `ip` + lowercased form/json field (empty field → IP only).
    IpAndInput {
        field: &'static str,
    },
}

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

/// Rate limiter plugin / route middleware.
pub struct RateLimit {
    max: usize,
    window: Duration,
    max_entries: usize,
    message: String,
    key: RateLimitKey,
    /// Optional prefix so presets (login/forgot) do not share buckets.
    prefix: Option<&'static str>,
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
            key: RateLimitKey::Ip,
            prefix: None,
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

    /// Built-in key strategy (default [`RateLimitKey::Ip`]).
    pub fn key(mut self, key: RateLimitKey) -> Self {
        self.key = key;
        self
    }

    /// Prefix prepended to the resolved key (`login:…`, `forgot:…`).
    pub fn prefix(mut self, prefix: &'static str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Custom rate-limit key (overrides [`Self::key`]).
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
            key: RateLimitKey::Ip,
            prefix: None,
            key_fn: None,
            skip: None,
            backend: Backend::SharedFixed(store),
        }
    }

    /// Login POST throttle: 5 / 60s, key = IP + email.
    pub fn login() -> Self {
        Self::per_minute(5)
            .prefix("login")
            .key(RateLimitKey::IpAndInput { field: "email" })
    }

    /// Forgot-password throttle: 5 / 60s, IP + email.
    pub fn forgot() -> Self {
        Self::per_minute(5)
            .prefix("forgot")
            .key(RateLimitKey::IpAndInput { field: "email" })
    }

    /// 2FA challenge throttle: 5 / 60s, IP.
    pub fn challenge() -> Self {
        Self::per_minute(5).prefix("2fa").key(RateLimitKey::Ip)
    }

    /// Email verification resend: 6 / 60s, IP + email.
    pub fn resend() -> Self {
        Self::new(6, Duration::from_secs(60))
            .prefix("verify-resend")
            .key(RateLimitKey::IpAndInput { field: "email" })
    }

    /// Route / mount middleware (same check as the plugin).
    pub fn middleware(self) -> MwEntry {
        let runtime = self.into_runtime();
        named(
            runtime.mw_name(),
            with_state(runtime, |rt, mut req, next| async move {
                if rt.skip.as_ref().is_some_and(|f| f(&req)) {
                    return next(req).await;
                }
                let key =
                    resolve_key(&mut req, rt.key_fn.as_ref(), rt.key, rt.prefix).await;
                let out = match &rt.backend {
                    RuntimeBackend::Sliding(w) => w.check(&key),
                    RuntimeBackend::Fixed(w) => w.check(&key).await,
                };
                if !out.allowed {
                    emit_exceeded(&req, &key, &out);
                    return limited(rt.message.as_str(), &out);
                }
                let mut res = next(req).await;
                attach_headers(&mut res, &out);
                res
            }),
        )
    }

    fn into_runtime(self) -> RateLimitRuntime {
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
        RateLimitRuntime {
            message: self.message,
            key: self.key,
            prefix: self.prefix,
            key_fn: self.key_fn,
            skip: self.skip,
            backend,
            name: self
                .prefix
                .map(|p| format!("rate-limit:{p}"))
                .unwrap_or_else(|| "rate-limit".into()),
        }
    }
}

struct RateLimitRuntime {
    message: String,
    key: RateLimitKey,
    prefix: Option<&'static str>,
    key_fn: Option<KeyFn>,
    skip: Option<SkipFn>,
    backend: RuntimeBackend,
    name: String,
}

impl RateLimitRuntime {
    fn mw_name(&self) -> String {
        self.name.clone()
    }
}

enum RuntimeBackend {
    Sliding(Arc<SlidingWindow>),
    Fixed(Arc<FixedWindow>),
}

impl Plugin for RateLimit {
    fn id(&self) -> &'static str {
        "rate-limit"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Rate limit")
            .description("Per-key request rate limiting")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        let runtime = self.into_runtime();
        app.use_middleware(named(
            runtime.mw_name(),
            with_state(runtime, |rt, mut req, next| async move {
                if rt.skip.as_ref().is_some_and(|f| f(&req)) {
                    return next(req).await;
                }
                let key =
                    resolve_key(&mut req, rt.key_fn.as_ref(), rt.key, rt.prefix).await;
                let out = match &rt.backend {
                    RuntimeBackend::Sliding(w) => w.check(&key),
                    RuntimeBackend::Fixed(w) => w.check(&key).await,
                };
                if !out.allowed {
                    emit_exceeded(&req, &key, &out);
                    return limited(rt.message.as_str(), &out);
                }
                let mut res = next(req).await;
                attach_headers(&mut res, &out);
                res
            }),
        ));
    }
}

fn emit_exceeded(req: &Request, key: &str, out: &Outcome) {
    if let Some(bus) = req.try_state::<EventBus>() {
        let now = unix_now();
        bus.dispatch(RateLimitExceeded {
            key: key.to_string(),
            limit: out.limit,
            retry_after: Some(out.reset.saturating_sub(now)),
        });
    }
}

async fn resolve_key(
    req: &mut Request,
    key_fn: Option<&KeyFn>,
    strategy: RateLimitKey,
    prefix: Option<&'static str>,
) -> String {
    let raw = if let Some(f) = key_fn {
        f(req)
    } else {
        match strategy {
            RateLimitKey::Ip => client_ip(req).to_string(),
            RateLimitKey::Identity => {
                if let Some(id) = req.get::<RateLimitIdentity>() {
                    format!("id:{}", id.0)
                } else {
                    client_ip(req).to_string()
                }
            }
            RateLimitKey::IpAndInput { field } => {
                let ip = client_ip(req);
                match peek_input_field(req, field).await {
                    Some(v) if !v.is_empty() => format!("{ip}:{}", v.to_ascii_lowercase()),
                    _ => ip.to_string(),
                }
            }
        }
    };
    match prefix {
        Some(p) => format!("{p}:{raw}"),
        None => raw,
    }
}

async fn peek_input_field(req: &mut Request, field: &str) -> Option<String> {
    let ct = req
        .header("content-type")
        .unwrap_or("")
        .to_ascii_lowercase();
    if ct.contains("application/json") {
        let Ok(v) = req.json::<JsonValue>().await else {
            return None;
        };
        return v
            .get(field)
            .and_then(|x| x.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    // form / multipart / urlencoded — cached on request via input()
    if let Ok(data) = req.input().await {
        return data
            .get(field)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    None
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
    use sova_store::MemoryStore;

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
