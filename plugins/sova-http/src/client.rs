//! Shared [`HttpClient`] and request builder.

use crate::breaker::{BreakerConfig, CircuitBreaker};
use crate::error::HttpError;
use crate::fake::FakeTransport;
use crate::reqwest_transport::{default_timeout, ReqwestTransport, DEFAULT_MAX_RESPONSE};
use crate::retry::{full_jitter, method_idempotent, parse_retry_after};
use crate::ssrf::SsrfPolicy;
use crate::transport::{OutRequest, OutResponse, Transport};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use sova_core::extend::BoxFuture;
use sova_core::{App, Plugin, Request};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::Instrument;

/// Per-named-client overrides from `sova.toml`.
#[derive(Debug, Clone, Default)]
pub struct NamedClientConfig {
    pub base_url: Option<String>,
    pub timeout: Option<Duration>,
    pub retries: Option<u32>,
    pub max_response_size: Option<usize>,
    pub deny_private_networks: Option<bool>,
    pub bearer_env: Option<String>,
    pub allow_hosts: Vec<String>,
}

/// Shared outbound HTTP handle stored in app state.
#[derive(Clone)]
pub struct HttpClient {
    transport: Arc<dyn Transport>,
    default_timeout: Duration,
    #[allow(dead_code)]
    max_response_size: usize,
    #[allow(dead_code)]
    ssrf: SsrfPolicy,
    breaker: Arc<CircuitBreaker>,
    named: Arc<HashMap<String, NamedClientConfig>>,
    fake: Option<FakeTransport>,
}

impl HttpClient {
    pub fn transport(&self) -> &Arc<dyn Transport> {
        &self.transport
    }

    pub fn fake(&self) -> Option<&FakeTransport> {
        self.fake.as_ref()
    }

    pub fn request(&self, method: Method, url: impl Into<String>) -> PendingRequest {
        PendingRequest::new(self.clone(), method, url.into(), None)
    }

    pub fn get(&self, url: impl Into<String>) -> PendingRequest {
        self.request(Method::GET, url)
    }

    pub fn post(&self, url: impl Into<String>) -> PendingRequest {
        self.request(Method::POST, url)
    }

    pub fn put(&self, url: impl Into<String>) -> PendingRequest {
        self.request(Method::PUT, url)
    }

    pub fn patch(&self, url: impl Into<String>) -> PendingRequest {
        self.request(Method::PATCH, url)
    }

    pub fn delete(&self, url: impl Into<String>) -> PendingRequest {
        self.request(Method::DELETE, url)
    }

    pub fn named(&self, name: &str) -> NamedClient {
        NamedClient {
            client: self.clone(),
            name: name.to_string(),
        }
    }
}

/// Bound to a named config (`[default.http.payments]`).
#[derive(Clone)]
pub struct NamedClient {
    client: HttpClient,
    name: String,
}

impl NamedClient {
    fn cfg(&self) -> NamedClientConfig {
        self.client
            .named
            .get(&self.name)
            .cloned()
            .unwrap_or_default()
    }

    fn resolve_url(&self, path: &str) -> Result<String, HttpError> {
        let cfg = self.cfg();
        let base = cfg
            .base_url
            .as_deref()
            .ok_or_else(|| HttpError::Other(format!("named http client `{}` missing base_url", self.name)))?;
        if path.starts_with("http://") || path.starts_with("https://") {
            return Ok(path.to_string());
        }
        Ok(format!(
            "{}/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        ))
    }

    pub fn get(&self, path: impl Into<String>) -> PendingRequest {
        self.request(Method::GET, path)
    }

    pub fn post(&self, path: impl Into<String>) -> PendingRequest {
        self.request(Method::POST, path)
    }

    pub fn put(&self, path: impl Into<String>) -> PendingRequest {
        self.request(Method::PUT, path)
    }

    pub fn patch(&self, path: impl Into<String>) -> PendingRequest {
        self.request(Method::PATCH, path)
    }

    pub fn delete(&self, path: impl Into<String>) -> PendingRequest {
        self.request(Method::DELETE, path)
    }

    pub fn request(&self, method: Method, path: impl Into<String>) -> PendingRequest {
        let path = path.into();
        let url = self.resolve_url(&path).unwrap_or(path);
        let cfg = self.cfg();
        let mut pending = PendingRequest::new(self.client.clone(), method, url, Some(self.name.clone()));
        if let Some(t) = cfg.timeout {
            pending.timeout = Some(t);
        }
        if let Some(n) = cfg.retries {
            pending.retries = Some(n);
        }
        if let Some(ref env_key) = cfg.bearer_env {
            if let Ok(token) = std::env::var(env_key) {
                pending = pending.bearer_auth(token);
            }
        }
        pending
    }
}

/// Fluent outbound request.
pub struct PendingRequest {
    client: HttpClient,
    method: Method,
    url: String,
    pub(crate) headers: HeaderMap,
    body: Option<Bytes>,
    timeout: Option<Duration>,
    retries: Option<u32>,
    idempotency_key: Option<String>,
    named: Option<String>,
    budget: Option<Duration>,
}

impl PendingRequest {
    fn new(client: HttpClient, method: Method, url: String, named: Option<String>) -> Self {
        Self {
            client,
            method,
            url,
            headers: HeaderMap::new(),
            body: None,
            timeout: None,
            retries: None,
            idempotency_key: None,
            named,
            budget: None,
        }
    }

    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_ref().as_bytes()),
            HeaderValue::from_str(value.as_ref()),
        ) {
            self.headers.insert(n, v);
        }
        self
    }

    pub fn bearer_auth(self, token: impl AsRef<str>) -> Self {
        self.header("authorization", format!("Bearer {}", token.as_ref()))
    }

    pub fn json<T: Serialize>(mut self, body: &T) -> Self {
        match serde_json::to_vec(body) {
            Ok(bytes) => {
                self.body = Some(Bytes::from(bytes));
                self.headers
                    .insert(http::header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
            }
            Err(e) => {
                tracing::error!("json serialize: {e}");
            }
        }
        self
    }

    pub fn body(mut self, bytes: impl Into<Bytes>) -> Self {
        self.body = Some(bytes.into());
        self
    }

    pub fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    /// Cap by remaining inbound request budget.
    pub fn with_budget(mut self, remaining: Option<Duration>) -> Self {
        self.budget = remaining;
        self
    }

    pub fn retry(mut self, times: u32) -> Self {
        self.retries = Some(times);
        self
    }

    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        self.headers.insert(
            HeaderName::from_static("idempotency-key"),
            HeaderValue::from_str(&key).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        self.idempotency_key = Some(key);
        self
    }

    fn effective_timeout(&self) -> Duration {
        let base = self
            .timeout
            .unwrap_or(self.client.default_timeout);
        match self.budget {
            Some(b) if b < base => b,
            _ => base,
        }
    }

    pub fn send(self) -> BoxFuture<Result<OutResponse, HttpError>> {
        Box::pin(self.send_inner())
    }

    async fn send_inner(self) -> Result<OutResponse, HttpError> {
        let host = url::Url::parse(&self.url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| self.url.clone());

        if !self.client.breaker.guard(&host) {
            return Err(HttpError::CircuitOpen(host));
        }

        let timeout = self.effective_timeout();
        if timeout.is_zero() {
            return Err(HttpError::Timeout);
        }

        let can_retry = method_idempotent(&self.method)
            || (self.method == Method::POST && self.idempotency_key.is_some());
        let max_retries = if can_retry {
            self.retries.unwrap_or(0)
        } else {
            0
        };

        let span = tracing::info_span!(
            "http.client",
            otel.kind = "client",
            http.method = %self.method,
            http.url = %self.url,
            http.host = %host,
            named = self.named.as_deref().unwrap_or(""),
        );

        async move {
            let mut attempt = 0u32;
            loop {
                let out = OutRequest {
                    method: self.method.clone(),
                    url: self.url.clone(),
                    headers: self.headers.clone(),
                    body: self.body.clone(),
                    timeout: Some(timeout),
                };
                let started = std::time::Instant::now();
                let result = self.client.transport.send(out).await;
                let elapsed = started.elapsed();

                match result {
                    Ok(res) => {
                        let code = res.status_u16();
                        tracing::info!(
                            status = code,
                            duration_ms = elapsed.as_millis() as u64,
                            "http.client done"
                        );
                        let retryable = code == 429 || (500..600).contains(&code);
                        if retryable && attempt < max_retries {
                            attempt += 1;
                            let delay = parse_retry_after(res.headers())
                                .unwrap_or_else(|| {
                                    full_jitter(
                                        Duration::from_millis(100),
                                        Duration::from_secs(2),
                                        attempt,
                                    )
                                });
                            self.client.breaker.record_failure(&host);
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                        if code >= 500 {
                            self.client.breaker.record_failure(&host);
                        } else {
                            self.client.breaker.record_success(&host);
                        }
                        return Ok(res);
                    }
                    Err(e) => {
                        tracing::info!(
                            error = %e,
                            duration_ms = elapsed.as_millis() as u64,
                            "http.client error"
                        );
                        let retryable = matches!(
                            e,
                            HttpError::Timeout | HttpError::Connect(_)
                        );
                        if retryable && attempt < max_retries {
                            attempt += 1;
                            self.client.breaker.record_failure(&host);
                            tokio::time::sleep(full_jitter(
                                Duration::from_millis(100),
                                Duration::from_secs(2),
                                attempt,
                            ))
                            .await;
                            continue;
                        }
                        self.client.breaker.record_failure(&host);
                        return Err(e);
                    }
                }
            }
        }
        .instrument(span)
        .await
    }
}

/// Plugin: shared client + request-id middleware.
pub struct Http {
    deny_private: bool,
    allow_hosts: Vec<String>,
    max_response_size: usize,
    default_timeout: Duration,
    breaker: BreakerConfig,
    fake: Option<FakeTransport>,
    named: HashMap<String, NamedClientConfig>,
}

impl Http {
    pub fn new() -> Self {
        Self {
            deny_private: true,
            allow_hosts: Vec::new(),
            max_response_size: DEFAULT_MAX_RESPONSE,
            default_timeout: default_timeout(),
            breaker: BreakerConfig::default(),
            fake: None,
            named: HashMap::new(),
        }
    }

    pub fn deny_private_networks(mut self) -> Self {
        self.deny_private = true;
        self
    }

    pub fn allow_private_networks(mut self) -> Self {
        self.deny_private = false;
        self
    }

    pub fn allow_hosts(mut self, hosts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.allow_hosts = hosts.into_iter().map(Into::into).collect();
        self
    }

    pub fn max_response_size(mut self, n: usize) -> Self {
        self.max_response_size = n;
        self
    }

    pub fn timeout(mut self, d: Duration) -> Self {
        self.default_timeout = d;
        self
    }

    pub fn fake() -> Self {
        let fake = FakeTransport::new();
        Self {
            fake: Some(fake),
            deny_private: false,
            ..Self::new()
        }
    }

    pub fn with_fake(mut self, fake: FakeTransport) -> Self {
        self.fake = Some(fake);
        self.deny_private = false;
        self
    }

    pub fn stub_get(mut self, url: impl Into<String>, body: impl Into<crate::fake::StubBody>) -> Self {
        if let Some(f) = self.fake.take() {
            self.fake = Some(f.get(url, body));
        }
        self
    }

    /// Alias for [`Self::stub_get`] (Laravel-style `Http::fake().get(...)`).
    pub fn get(self, url: impl Into<String>, body: impl Into<crate::fake::StubBody>) -> Self {
        self.stub_get(url, body)
    }

    pub fn stub_post(mut self, url: impl Into<String>, status: u16) -> Self {
        if let Some(f) = self.fake.take() {
            self.fake = Some(f.post(url, status));
        }
        self
    }

    /// Alias for [`Self::stub_post`].
    pub fn post(self, url: impl Into<String>, status: u16) -> Self {
        self.stub_post(url, status)
    }

    pub fn stub_fail(mut self, url: impl Into<String>, msg: impl Into<String>) -> Self {
        if let Some(f) = self.fake.take() {
            self.fake = Some(f.fail(url, msg));
        }
        self
    }

    /// Alias for [`Self::stub_fail`].
    pub fn fail(self, url: impl Into<String>, msg: impl Into<String>) -> Self {
        self.stub_fail(url, msg)
    }

    /// Merge named clients from [`sova_core::ConfigDoc`] already on `app`.
    pub fn with_config_from_app(mut self, app: &App) -> Self {
        if let Some(doc) = app.config_doc() {
            for (name, table) in doc.http_clients() {
                self.named.insert(name, parse_named(&table));
            }
        }
        self
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_named(table: &toml::map::Map<String, toml::Value>) -> NamedClientConfig {
    use sova_core::extend::parse_bytes;
    use sova_core::extend::parse_duration;

    let mut cfg = NamedClientConfig::default();
    if let Some(v) = table.get("base_url").and_then(|v| v.as_str()) {
        cfg.base_url = Some(v.to_string());
    }
    if let Some(v) = table.get("timeout").and_then(|v| v.as_str()) {
        cfg.timeout = parse_duration(v).ok();
    }
    if let Some(v) = table.get("retries").and_then(|v| v.as_integer()) {
        cfg.retries = Some(v as u32);
    }
    if let Some(v) = table.get("max_response_size").and_then(|v| v.as_str()) {
        cfg.max_response_size = parse_bytes(v).ok();
    }
    if let Some(v) = table.get("deny_private_networks").and_then(|v| v.as_bool()) {
        cfg.deny_private_networks = Some(v);
    }
    if let Some(v) = table.get("bearer_env").and_then(|v| v.as_str()) {
        cfg.bearer_env = Some(v.to_string());
    }
    if let Some(toml::Value::Array(arr)) = table.get("allow_hosts") {
        cfg.allow_hosts = arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
    }
    cfg
}

impl Plugin for Http {
    fn id(&self) -> &'static str {
        "http"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("HTTP client")
            .description("Outbound HTTP client with SSRF guards and named configs")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        let mut named_clients = self.named;
        if let Some(doc) = app.config_doc() {
            for (name, table) in doc.http_clients() {
                named_clients
                    .entry(name)
                    .or_insert_with(|| parse_named(&table));
            }
        }

        let ssrf = SsrfPolicy {
            deny_private: self.deny_private,
            allow_hosts: self.allow_hosts,
        };

        let (transport, fake): (Arc<dyn Transport>, Option<FakeTransport>) =
            if let Some(fake) = self.fake.clone() {
                (Arc::new(fake.clone()) as Arc<dyn Transport>, Some(fake))
            } else {
                let t = ReqwestTransport::new(ssrf.clone(), self.max_response_size)
                    .expect("reqwest client");
                (Arc::new(t) as Arc<dyn Transport>, None)
            };

        let client = HttpClient {
            transport,
            default_timeout: self.default_timeout,
            max_response_size: self.max_response_size,
            ssrf,
            breaker: Arc::new(CircuitBreaker::new(self.breaker)),
            named: Arc::new(named_clients),
            fake,
        };
        app.state(client);
        // RequestId comes from core `request_id()` middleware (presets / app).
    }
}

pub(crate) fn propagation_headers(req: &Request) -> HeaderMap {
    let mut h = HeaderMap::new();
    if let Some(tp) = req.header("traceparent") {
        if let Ok(v) = HeaderValue::from_str(tp) {
            h.insert(HeaderName::from_static("traceparent"), v);
        }
    }
    if let Some(id) = req
        .get::<sova_core::RequestId>()
        .map(|r| r.0.as_str())
        .or_else(|| req.header("x-request-id"))
    {
        if let Ok(v) = HeaderValue::from_str(id) {
            h.insert(HeaderName::from_static("x-request-id"), v);
        }
    }
    if let Some(al) = req.header("accept-language") {
        if let Ok(v) = HeaderValue::from_str(al) {
            h.insert(http::header::ACCEPT_LANGUAGE, v);
        }
    }
    h
}
