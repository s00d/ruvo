mod hooks;

pub(crate) use hooks::{AppInner, ShutdownHook, StartupHook};
pub use hooks::Server;

use crate::error::{Error, Result};
use crate::plugin::Plugin;
use crate::request::Request;
use crate::response::Response;
use crate::router::Router;
use crate::server;
use crate::state::StateMap;
use bytes::Bytes;
use http::Method;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
#[cfg(unix)]
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_MAX_BODY: usize = 2 * 1024 * 1024;
const DEFAULT_MAX_CONNECTIONS: usize = 1024;
const DEFAULT_MAX_HEADERS: usize = 100;
/// Default drain budget — leave headroom under k8s' 30s `terminationGracePeriodSeconds`.
const DEFAULT_DRAIN_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);
/// Keep-alive idle wait between requests (also Slowloris / first-header wait via hyper).
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Thin wrapper over [`Router`] plus server settings and lifecycle hooks.
pub struct App {
    router: Router,
    max_body_size: usize,
    max_connections: usize,
    max_headers: usize,
    max_buf_size: Option<usize>,
    request_timeout: Option<Duration>,
    header_read_timeout: Duration,
    idle_timeout: Duration,
    drain_timeout: Duration,
    keep_alive: bool,
    listen_addr: Option<SocketAddr>,
    trust_proxy: bool,
    on_startup: Vec<StartupHook>,
    on_shutdown: Vec<ShutdownHook>,
}

impl App {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            max_body_size: DEFAULT_MAX_BODY,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            max_headers: DEFAULT_MAX_HEADERS,
            max_buf_size: None,
            request_timeout: Some(Duration::from_secs(30)),
            header_read_timeout: DEFAULT_HEADER_READ_TIMEOUT,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            drain_timeout: DEFAULT_DRAIN_TIMEOUT,
            keep_alive: true,
            listen_addr: None,
            trust_proxy: false,
            on_startup: Vec::new(),
            on_shutdown: Vec::new(),
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

    /// Bind address for [`Self::listen`] (default `0.0.0.0:port` — IPv4 only;
    /// use `[::]` / `listen_str` for dual-stack where the OS supports it).
    pub fn listen_addr(&mut self, addr: SocketAddr) -> &mut Self {
        self.listen_addr = Some(addr);
        self
    }

    /// When true, `ClientAddr` may use `X-Forwarded-For` / `Forwarded` (only behind a trusted proxy).
    pub fn trust_proxy(&mut self, trust: bool) -> &mut Self {
        self.trust_proxy = trust;
        self
    }

    pub fn install<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        plugin.install(self);
        self
    }

    /// Run before accepting connections. `Err` prevents the server from starting.
    pub fn on_startup<F, Fut>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(Arc<StateMap>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.on_startup
            .push(Box::new(move |state| Box::pin(f(state))));
        self
    }

    /// Run after the accept loop stops and connections drain.
    pub fn on_shutdown<F, Fut>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.on_shutdown.push(Box::new(move || Box::pin(f())));
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

    /// Run startup hooks only (for tests). Uses a clone of router state.
    #[cfg(any(test, feature = "testing"))]
    pub async fn run_startup(&mut self) -> Result<Arc<StateMap>> {
        let state = Arc::new(self.router.state.clone_map());
        let hooks = std::mem::take(&mut self.on_startup);
        for hook in hooks {
            hook(Arc::clone(&state)).await?;
        }
        Ok(state)
    }

    /// Run shutdown hooks only (for tests).
    #[cfg(any(test, feature = "testing"))]
    pub async fn run_shutdown(&mut self) {
        let hooks = std::mem::take(&mut self.on_shutdown);
        for hook in hooks {
            hook().await;
        }
    }

    pub async fn listen(self, port: u16) -> Result<()> {
        server::listen(self, Some(port), None, None).await
    }

    /// Listen on an explicit socket address.
    pub async fn listen_on(self, addr: SocketAddr) -> Result<()> {
        server::listen(self, None, Some(addr), None).await
    }

    /// Parse `"host:port"` / `"[::1]:3000"` and listen.
    pub async fn listen_str(self, addr: &str) -> Result<()> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|e| Error::Internal(format!("listen_str: invalid address {addr:?}: {e}")))?;
        self.listen_on(addr).await
    }

    /// `PORT` from the environment (Heroku/Railway/Fly/Cloud Run), else `default_port`.
    /// Optional `HOST` (IP; default `0.0.0.0`).
    pub async fn listen_env(self, default_port: u16) -> Result<()> {
        let addr = addr_from_env(default_port)?;
        self.listen_on(addr).await
    }

    /// Like [`listen`](Self::listen), but also stops when `shutdown` completes
    /// (in addition to Ctrl-C / SIGTERM).
    pub async fn listen_with_shutdown<F>(self, port: u16, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        server::listen(self, Some(port), None, Some(Box::pin(shutdown))).await
    }

    /// Like [`listen_on`](Self::listen_on) with a programmatic shutdown future.
    pub async fn listen_on_with_shutdown<F>(self, addr: SocketAddr, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        server::listen(self, None, Some(addr), Some(Box::pin(shutdown))).await
    }

    /// Listen on an already-bound [`tokio::net::TcpListener`].
    pub async fn listen_listener(self, listener: tokio::net::TcpListener) -> Result<()> {
        server::listen_with_listener(self, listener, None).await
    }

    /// Listen on a bound TCP listener with programmatic shutdown.
    pub async fn listen_listener_with_shutdown<F>(
        self,
        listener: tokio::net::TcpListener,
        shutdown: F,
    ) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        server::listen_with_listener(self, listener, Some(Box::pin(shutdown))).await
    }

    /// Listen on a Unix domain socket (behind nginx / local IPC). Unix only.
    #[cfg(unix)]
    pub async fn listen_uds(self, path: impl AsRef<Path>) -> Result<()> {
        server::listen_uds(self, path.as_ref(), None).await
    }

    /// Unix domain socket + programmatic shutdown.
    #[cfg(unix)]
    pub async fn listen_uds_with_shutdown<F>(self, path: impl AsRef<Path>, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        server::listen_uds(self, path.as_ref(), Some(Box::pin(shutdown))).await
    }
}

fn addr_from_env(default_port: u16) -> Result<SocketAddr> {
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
