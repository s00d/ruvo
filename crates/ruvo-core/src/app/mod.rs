mod bind;
mod hooks;

pub(crate) use hooks::{AppInner, ListenParts, ShutdownHook, StartupHook};
pub use bind::{Bind, BoundApp};
pub use bind::Http;
pub use hooks::Server;

use crate::error::{Error, Result};
use crate::plugin::Plugin;
use crate::request::Request;
use crate::response::Response;
use crate::router::Router;
use crate::service::{BackgroundService, BoxedService};
use crate::state::StateMap;
use bytes::Bytes;
use http::Method;
use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_MAX_BODY: usize = 2 * 1024 * 1024;
const DEFAULT_MAX_CONNECTIONS: usize = 1024;
const DEFAULT_MAX_UPGRADED: usize = 1024;
const DEFAULT_MAX_CONCURRENT_STREAMS: usize = 200;
const DEFAULT_MAX_HEADERS: usize = 100;
/// Default drain budget — leave headroom under k8s' 30s `terminationGracePeriodSeconds`.
const DEFAULT_DRAIN_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);
/// Keep-alive idle wait between requests (also Slowloris / first-header wait via hyper).
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Thin wrapper over [`Router`] plus server settings and lifecycle hooks.
pub struct App {
    pub(crate) router: Router,
    pub(crate) max_body_size: usize,
    pub(crate) max_connections: usize,
    pub(crate) max_upgraded_connections: usize,
    pub(crate) max_concurrent_streams: usize,
    pub(crate) max_headers: usize,
    pub(crate) max_buf_size: Option<usize>,
    pub(crate) request_timeout: Option<Duration>,
    pub(crate) header_read_timeout: Duration,
    pub(crate) idle_timeout: Duration,
    pub(crate) drain_timeout: Duration,
    pub(crate) keep_alive: bool,
    pub(crate) listen_addr: Option<SocketAddr>,
    pub(crate) trust_proxy: bool,
    pub(crate) reuseport: bool,
    /// Set by CLI listen helpers — BackgroundServices skipped unless `service_in_cli`.
    pub(crate) cli_mode: bool,
    pub(crate) service_in_cli: bool,
    pub(crate) hsts: bool,
    pub(crate) alt_svc: Option<String>,
    pub(crate) on_startup: Vec<StartupHook>,
    pub(crate) on_shutdown: Vec<ShutdownHook>,
    pub(crate) services: Vec<BoxedService>,
}

impl App {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            max_body_size: DEFAULT_MAX_BODY,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            max_upgraded_connections: DEFAULT_MAX_UPGRADED,
            max_concurrent_streams: DEFAULT_MAX_CONCURRENT_STREAMS,
            max_headers: DEFAULT_MAX_HEADERS,
            max_buf_size: None,
            request_timeout: Some(Duration::from_secs(30)),
            header_read_timeout: DEFAULT_HEADER_READ_TIMEOUT,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            drain_timeout: DEFAULT_DRAIN_TIMEOUT,
            keep_alive: true,
            listen_addr: None,
            trust_proxy: false,
            reuseport: false,
            cli_mode: false,
            service_in_cli: false,
            hsts: false,
            alt_svc: None,
            on_startup: Vec::new(),
            on_shutdown: Vec::new(),
            services: Vec::new(),
        }
    }

    pub fn max_body_size(&mut self, bytes: usize) -> &mut Self {
        self.max_body_size = bytes;
        self
    }

    /// Cap concurrent TCP/UDS connections (default 1024).
    pub fn max_connections(&mut self, n: usize) -> &mut Self {
        self.max_connections = n.max(1);
        self
    }

    /// Cap concurrent HTTP upgrades (WebSocket, …). Excess → **503** + `Retry-After`.
    pub fn max_upgraded_connections(&mut self, n: usize) -> &mut Self {
        self.max_upgraded_connections = n.max(1);
        self
    }

    /// Cap concurrent HTTP/2 streams per connection (default 200).
    /// Excess streams → `GOAWAY`/stream-level rejection handled by hyper.
    pub fn max_concurrent_streams(&mut self, n: usize) -> &mut Self {
        self.max_concurrent_streams = n.max(1);
        self
    }

    /// Max HTTP/1 header count (default 100). Excess → 431 from hyper.
    pub fn max_headers(&mut self, n: usize) -> &mut Self {
        self.max_headers = n.max(1);
        self
    }

    /// Cap hyper's connection buffer (headers + body framing). Minimum 8192.
    /// Default ~400 KiB. Use this to bound oversized header blocks.
    pub fn max_buf_size(&mut self, bytes: usize) -> &mut Self {
        self.max_buf_size = Some(bytes.max(8192));
        self
    }

    /// Per-request timeout around the handler (default 30s). `None` disables.
    ///
    /// Measured: timeout ends when the handler returns a [`Response`]. Streaming
    /// response bodies (SSE) continue afterward and are **not** cut by this timer.
    /// Idle between stream chunks is governed by TCP/keep-alive, not this setting.
    pub fn request_timeout(&mut self, timeout: Option<Duration>) -> &mut Self {
        self.request_timeout = timeout;
        self
    }

    /// Timeout for reading request headers (Slowloris). Also applied while waiting
    /// for the next keep-alive request (see [`Self::idle_timeout`]). Default 10s.
    pub fn header_read_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.header_read_timeout = timeout;
        self
    }

    /// Keep-alive idle: how long a quiet connection may wait for the next request.
    /// Hyper uses one timer for header reads; the effective wait is
    /// `min(header_read_timeout, idle_timeout)`. Default 60s.
    pub fn idle_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.idle_timeout = timeout;
        self
    }

    /// How long to wait for in-flight connections after accept stops (default 20s).
    pub fn drain_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.drain_timeout = timeout;
        self
    }

    /// HTTP/1 keep-alive (default `true`).
    pub fn keep_alive(&mut self, enabled: bool) -> &mut Self {
        self.keep_alive = enabled;
        self
    }

    /// Bind address for [`Self::bind`](App::bind) (default `0.0.0.0:port` — IPv4 only;
    /// use [`Bind::Str`] or [`Bind::Addr`] with `[::]:port` for dual-stack where the OS supports it).
    pub fn listen_addr(&mut self, addr: SocketAddr) -> &mut Self {
        self.listen_addr = Some(addr);
        self
    }

    /// When true, `ClientAddr` may use `X-Forwarded-For` / `Forwarded` (only behind a trusted proxy).
    pub fn trust_proxy(&mut self, trust: bool) -> &mut Self {
        self.trust_proxy = trust;
        self
    }

    /// Enable `SO_REUSEPORT` on TCP bind (requires feature `listen-reuseport`).
    pub fn listen_reuseport(&mut self, enabled: bool) -> &mut Self {
        self.reuseport = enabled;
        self
    }

    /// Mark this app as running under the CLI helper (skips BackgroundServices by default).
    pub fn cli_mode(&mut self, enabled: bool) -> &mut Self {
        self.cli_mode = enabled;
        self
    }

    /// Start BackgroundServices even when [`Self::cli_mode`] is set (default `false`).
    pub fn service_in_cli(&mut self, enabled: bool) -> &mut Self {
        self.service_in_cli = enabled;
        self
    }

    pub fn install<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        plugin.install(self);
        self
    }

    /// Register a process-local [`BackgroundService`].
    ///
    /// Lifecycle: `compile → on_startup → services → accept`;
    /// stop: `stop accept → drain → stop services → on_shutdown`.
    pub fn service<S: BackgroundService + 'static>(&mut self, service: S) -> &mut Self {
        self.services.push(Box::new(service));
        self
    }

    /// Run before accepting connections. `Err` prevents the server from starting.
    pub fn on_startup<F, Fut>(&mut self, f: F) -> &mut Self
    where
        F: Fn(Arc<StateMap>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.on_startup
            .push(Arc::new(move |state| Box::pin(f(state))));
        self
    }

    /// Run after the accept loop stops, connections drain, and services stop.
    pub fn on_shutdown<F, Fut>(&mut self, f: F) -> &mut Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.on_shutdown
            .push(Arc::new(move || Box::pin(f())));
        self
    }

    /// Route map for debugging / startup banner.
    pub fn explain(&self) -> String {
        self.router.explain()
    }

    /// Handle one request (compiles the router each call). Prefer [`Self::build`].
    pub async fn handle(&self, req: Request) -> Response {
        match self.build() {
            Ok(server) => server.handle(req).await,
            Err(err) => err.into_response(),
        }
    }

    /// Sugar over [`Request::builder`] + [`Self::handle`] (no custom headers).
    /// For headers use `Request::builder().header(...).build()` + [`Self::handle`].
    /// Prefer [`Server::handle_request`] after [`Self::build`].
    pub async fn handle_request(&self, method: Method, path: &str, body: &str) -> Response {
        let req = Request::builder()
            .method(method)
            .path(path)
            .body(Bytes::from(body.to_string()))
            .build();
        self.handle(req).await
    }

    /// Run startup hooks via [`Server::run_startup`] (non-destructive).
    #[cfg(any(test, feature = "testing"))]
    pub async fn run_startup(&self) -> Result<Arc<StateMap>> {
        self.build()?.run_startup().await
    }

    /// Run shutdown hooks via [`Server::run_shutdown`] (non-destructive).
    #[cfg(any(test, feature = "testing"))]
    pub async fn run_shutdown(&self) {
        if let Ok(server) = self.build() {
            server.run_shutdown().await;
        }
    }
}

pub(crate) fn addr_from_env(default_port: u16) -> Result<SocketAddr> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(default_port);

    match std::env::var("HOST") {
        Ok(host) if !host.is_empty() => {
            if let Ok(ip) = host.parse::<IpAddr>() {
                return Ok(SocketAddr::new(ip, port));
            }
            // Allow HOST="127.0.0.1:3000" to override port entirely.
            if let Ok(addr) = host.parse::<SocketAddr>() {
                return Ok(addr);
            }
            format!("{host}:{port}")
                .parse()
                .map_err(|e| Error::Internal(format!("HOST={host:?} invalid: {e}")))
        }
        _ => Ok(SocketAddr::from(([0, 0, 0, 0], port))),
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for App {
    type Target = Router;

    fn deref(&self) -> &Router {
        &self.router
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Router {
        &mut self.router
    }
}

#[cfg(test)]
mod env_addr_tests {
    use super::addr_from_env;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn port_from_env() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("PORT", "9876");
        std::env::remove_var("HOST");
        let addr = addr_from_env(3000).unwrap();
        assert_eq!(addr.port(), 9876);
        std::env::remove_var("PORT");
    }

    #[test]
    fn host_ip_from_env() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PORT");
        std::env::set_var("HOST", "127.0.0.1");
        let addr = addr_from_env(3000).unwrap();
        assert_eq!(addr, "127.0.0.1:3000".parse().unwrap());
        std::env::remove_var("HOST");
    }
}
