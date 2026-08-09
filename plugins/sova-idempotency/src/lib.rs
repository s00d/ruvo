//! Inbound `Idempotency-Key` middleware for mutating HTTP methods.

use bytes::Bytes;
use http::Method;
use serde::{Deserialize, Serialize};
use sova_core::extend::{named, MwEntry};
use sova_core::{with_state, App, Plugin, Request, Response};
use sova_store::{AppStore, KvStore};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const DEFAULT_MAX_BODY: usize = 256 * 1024;

/// Replay cached successful responses for the same `Idempotency-Key`.
pub struct Idempotency {
    store: Arc<dyn KvStore>,
    ttl: Duration,
    max_body: usize,
    prefix: String,
}

#[derive(Serialize, Deserialize)]
struct CachedResponse {
    status: u16,
    content_type: Option<String>,
    body: Vec<u8>,
}

impl Idempotency {
    pub fn from_store(store: Arc<dyn KvStore>) -> Self {
        Self {
            store,
            ttl: DEFAULT_TTL,
            max_body: DEFAULT_MAX_BODY,
            prefix: "idem:".into(),
        }
    }

    /// Use installed [`AppStore`] (`idem` namespace). Panics if SharedStore is missing.
    pub fn from_app(app: &App) -> Self {
        let store = app.try_state::<AppStore>().unwrap_or_else(|| {
            panic!("Idempotency::from_app requires SharedStore / AppStore installed first")
        });
        Self::from_store(store.namespaced("idem"))
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn max_body(mut self, bytes: usize) -> Self {
        self.max_body = bytes.max(1);
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Route / mount middleware (same as plugin install).
    pub fn middleware(self) -> MwEntry {
        named(
            "idempotency",
            with_state(self, |rt, req, next| async move {
                run(rt, req, next).await
            }),
        )
    }

    /// Prefer an installed [`AppStore`], else fall back to `store`.
    pub fn from_app_or(app: &App, store: Arc<dyn KvStore>) -> Self {
        let store = app
            .try_state::<AppStore>()
            .map(|s| Arc::clone(&s.inner))
            .unwrap_or(store);
        Self::from_store(store)
    }
}

impl Plugin for Idempotency {
    fn id(&self) -> &'static str {
        "idempotency"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Idempotency")
            .description("Replay 2xx responses for Idempotency-Key on mutating methods")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.use_middleware(self.middleware());
    }
}

fn is_mutating(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

async fn run(
    rt: Arc<Idempotency>,
    req: Request,
    next: sova_core::Next,
) -> Response {
    if !is_mutating(&req.method) {
        return next(req).await;
    }
    let Some(key) = req
        .header("idempotency-key")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
    else {
        return next(req).await;
    };

    let cache_key = format!("{}{}", rt.prefix, key);
    if let Some(bytes) = rt.store.get(&cache_key).await {
        if let Ok(cached) = serde_json::from_slice::<CachedResponse>(&bytes) {
            let mut res = Response::bytes(cached.body, cached.content_type.as_deref().unwrap_or("application/octet-stream"))
                .status(cached.status)
                .header("x-idempotency-replay", "true");
            if let Some(ct) = cached.content_type {
                res = res.header("content-type", ct);
            }
            return res;
        }
    }

    let res = next(req).await;
    let status = res.status_code().as_u16();
    if !(200..300).contains(&status) {
        return res;
    }

    let Some(body) = res.body_bytes().map(|b| b.to_vec()) else {
        return res;
    };
    if body.len() > rt.max_body {
        return res;
    }

    let content_type = res
        .headers()
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let payload = CachedResponse {
        status,
        content_type,
        body,
    };
    if let Ok(raw) = serde_json::to_vec(&payload) {
        rt.store
            .set(&cache_key, Bytes::from(raw), Some(rt.ttl))
            .await;
    }
    res.header("x-idempotency-replay", "false")
}
