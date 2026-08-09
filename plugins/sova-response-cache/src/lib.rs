//! GET response cache with optional private caching and prefix invalidation.

use bytes::Bytes;
use http::Method;
use serde::{Deserialize, Serialize};
use sova_core::extend::{named, MwEntry};
use sova_core::{with_state, App, Plugin, Request, Response};
use sova_store::KvStore;
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_TTL: Duration = Duration::from_secs(60);
const DEFAULT_MAX_BODY: usize = 512 * 1024;

/// Cache successful GET responses in a [`KvStore`].
#[derive(Clone)]
pub struct ResponseCache {
    store: Arc<dyn KvStore>,
    ttl: Duration,
    max_body: usize,
    vary: Vec<String>,
    cache_private: bool,
    prefix: String,
}

#[derive(Serialize, Deserialize)]
struct Cached {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl ResponseCache {
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self {
            store,
            ttl: DEFAULT_TTL,
            max_body: DEFAULT_MAX_BODY,
            vary: Vec::new(),
            cache_private: false,
            prefix: "rcache:".into(),
        }
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn max_body(mut self, bytes: usize) -> Self {
        self.max_body = bytes.max(1);
        self
    }

    pub fn vary(mut self, headers: &[&str]) -> Self {
        self.vary = headers.iter().map(|s| s.to_ascii_lowercase()).collect();
        self
    }

    /// Allow caching requests that carry `Authorization` / `Cookie`.
    pub fn cache_private(mut self, yes: bool) -> Self {
        self.cache_private = yes;
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Invalidate keys under `path_prefix` (same as [`ResponseCacheHandle::invalidate_prefix`]).
    ///
    /// Typical pattern with [`sova_core::EventBus`]:
    /// ```ignore
    /// let handle = app.try_state::<ResponseCacheHandle>().unwrap();
    /// bus.listen::<NoteCreated, _>(move |_| {
    ///     let h = handle.clone();
    ///     tokio::spawn(async move { h.invalidate_prefix("/api/notes").await; });
    /// });
    /// ```
    pub async fn invalidate_prefix(&self, path_prefix: &str) -> u64 {
        let full = format!("{}{}", self.prefix, path_prefix);
        self.store.clear_prefix(&full).await
    }

    pub fn middleware(self) -> MwEntry {
        named(
            "response-cache",
            with_state(self, |rt, req, next| async move { run(rt, req, next).await }),
        )
    }

    fn cache_key(&self, req: &Request) -> String {
        let mut parts = vec![req.method.as_str().to_string(), req.path.clone()];
        let mut vary_vals: Vec<(String, String)> = self
            .vary
            .iter()
            .map(|h| {
                (
                    h.clone(),
                    req.header(h).unwrap_or("").to_string(),
                )
            })
            .collect();
        vary_vals.sort_by(|a, b| a.0.cmp(&b.0));
        for (h, v) in vary_vals {
            parts.push(format!("{h}={v}"));
        }
        format!("{}{}", self.prefix, parts.join("|"))
    }
}

impl Plugin for ResponseCache {
    fn id(&self) -> &'static str {
        "response-cache"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Response cache")
            .description("Cache GET 200 responses in KvStore")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.state(self.clone_handle());
        app.use_middleware(self.middleware());
    }
}

impl ResponseCache {
    fn clone_handle(&self) -> ResponseCacheHandle {
        ResponseCacheHandle {
            store: Arc::clone(&self.store),
            prefix: self.prefix.clone(),
        }
    }
}

/// App-state handle for invalidation from handlers / event listeners.
#[derive(Clone)]
pub struct ResponseCacheHandle {
    store: Arc<dyn KvStore>,
    prefix: String,
}

impl ResponseCacheHandle {
    pub async fn invalidate_prefix(&self, path_prefix: &str) -> u64 {
        let full = format!("{}{}", self.prefix, path_prefix);
        self.store.clear_prefix(&full).await
    }
}

async fn run(rt: Arc<ResponseCache>, req: Request, next: sova_core::Next) -> Response {
    if req.method != Method::GET {
        return next(req).await;
    }
    if !rt.cache_private
        && (req.header("authorization").is_some() || req.header("cookie").is_some())
    {
        return next(req).await;
    }

    let key = rt.cache_key(&req);
    if let Some(bytes) = rt.store.get(&key).await {
        if let Ok(cached) = serde_json::from_slice::<Cached>(&bytes) {
            let mut res = Response::bytes(cached.body, "application/octet-stream").status(cached.status);
            for (n, v) in cached.headers {
                res = res.header(n, v);
            }
            return res
                .header("x-cache", "HIT")
                .header("cache-control", format!("max-age={}", rt.ttl.as_secs()));
        }
    }

    let res = next(req).await;
    if res.status_code().as_u16() != 200 {
        return res.header("x-cache", "MISS");
    }
    let Some(body) = res.body_bytes().map(|b| b.to_vec()) else {
        return res.header("x-cache", "MISS");
    };
    if body.len() > rt.max_body {
        return res.header("x-cache", "MISS");
    }

    let keep = ["content-type", "content-language", "etag"];
    let headers: Vec<(String, String)> = res
        .headers()
        .iter()
        .filter_map(|(k, v)| {
            let name = k.as_str().to_ascii_lowercase();
            if keep.contains(&name.as_str()) {
                v.to_str().ok().map(|val| (name, val.to_string()))
            } else {
                None
            }
        })
        .collect();

    let payload = Cached {
        status: 200,
        headers,
        body,
    };
    if let Ok(raw) = serde_json::to_vec(&payload) {
        rt.store.set(&key, Bytes::from(raw), Some(rt.ttl)).await;
    }
    res.header("x-cache", "MISS")
        .header("cache-control", format!("public, max-age={}", rt.ttl.as_secs()))
}
