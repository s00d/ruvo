use super::App;
use crate::error::Result;
use crate::handler::BoxFuture;
use crate::request::Request;
use crate::response::Response;
use crate::router::{compile_router, CompiledRouter};
use crate::service::BoxedService;
use crate::state::StateMap;
use crate::upgrade::UpgradeBudget;
use bytes::Bytes;
use http::Method;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

pub(crate) type StartupHook =
    Arc<dyn Fn(Arc<StateMap>) -> BoxFuture<Result<()>> + Send + Sync>;
pub(crate) type ShutdownHook = Arc<dyn Fn() -> BoxFuture<()> + Send + Sync>;

/// Compiled app ready to handle requests without recompiling the router.
#[derive(Clone)]
pub struct Server {
    pub(crate) inner: Arc<AppInner>,
    #[allow(dead_code)] // used under feature `testing`
    pub(crate) startups: Vec<StartupHook>,
    #[allow(dead_code)]
    pub(crate) shutdowns: Vec<ShutdownHook>,
}

impl Server {
    /// Shared application state map (same as request `state`).
    pub fn state(&self) -> Arc<StateMap> {
        self.inner.state()
    }

    /// Run startup hooks without consuming them (safe to call multiple times).
    #[cfg(any(test, feature = "testing"))]
    pub async fn run_startup(&self) -> Result<Arc<StateMap>> {
        let state = self.state();
        for hook in &self.startups {
            hook(Arc::clone(&state)).await?;
        }
        Ok(state)
    }

    /// Run shutdown hooks without consuming them.
    #[cfg(any(test, feature = "testing"))]
    pub async fn run_shutdown(&self) {
        for hook in &self.shutdowns {
            hook().await;
        }
    }

    /// Handle a request (tests / embedded). Injects router state.
    pub async fn handle(&self, mut req: Request) -> Response {
        req.state = Arc::clone(&self.inner.compiled.state);
        self.inner.compiled.dispatch(req).await
    }

    /// Convenience for unit tests without headers.
    /// Custom headers: [`Request::builder`] + [`Self::handle`].
    pub async fn handle_request(&self, method: Method, path: &str, body: &str) -> Response {
        let req = Request::builder()
            .method(method)
            .path(path)
            .body(Bytes::from(body.to_string()))
            .build();
        self.handle(req).await
    }
}

pub(crate) struct ListenParts {
    pub(crate) inner: AppInner,
    pub(crate) startups: Vec<StartupHook>,
    pub(crate) shutdowns: Vec<ShutdownHook>,
    pub(crate) services: Vec<BoxedService>,
    /// When false (CLI default), BackgroundServices are not started.
    pub(crate) start_services: bool,
}

impl App {
    /// Compile routes once into a [`Server`]. Prefer this over repeated [`App::handle`].
    pub fn build(&self) -> Result<Server> {
        let router = self.router.clone_for_compile();
        let explain = router.explain();
        let route_count = router.route_entries().len();
        let compiled = Arc::new(compile_router(router)?);
        Ok(Server {
            inner: Arc::new(AppInner::from_settings(
                compiled,
                route_count,
                explain,
                AppSettings::from(self),
            )),
            startups: self.on_startup.clone(),
            shutdowns: self.on_shutdown.clone(),
        })
    }

    pub(crate) fn into_listen_parts(mut self) -> Result<ListenParts> {
        let services = std::mem::take(&mut self.services);
        let startups = self.on_startup.clone();
        let shutdowns = self.on_shutdown.clone();
        let start_services = !self.cli_mode || self.service_in_cli;

        let explain = self.router.explain();
        let route_count = self.router.route_entries().len();
        let settings = AppSettings::from(&self);
        let router = self.router;
        let compiled = Arc::new(compile_router(router)?);

        Ok(ListenParts {
            inner: AppInner::from_settings(compiled, route_count, explain, settings),
            startups,
            shutdowns,
            services,
            start_services,
        })
    }
}

pub(crate) struct AppSettings {
    pub max_body_size: usize,
    pub max_connections: usize,
    pub max_upgraded_connections: usize,
    pub max_concurrent_streams: usize,
    pub max_headers: usize,
    pub max_buf_size: Option<usize>,
    pub request_timeout: Option<Duration>,
    pub header_read_timeout: Duration,
    pub idle_timeout: Duration,
    pub drain_timeout: Duration,
    pub keep_alive: bool,
    pub listen_addr: Option<SocketAddr>,
    pub trust_proxy: bool,
    pub reuseport: bool,
    pub hsts: bool,
    pub alt_svc: Option<String>,
}

impl From<&App> for AppSettings {
    fn from(app: &App) -> Self {
        Self {
            max_body_size: app.max_body_size,
            max_connections: app.max_connections,
            max_upgraded_connections: app.max_upgraded_connections,
            max_concurrent_streams: app.max_concurrent_streams,
            max_headers: app.max_headers,
            max_buf_size: app.max_buf_size,
            request_timeout: app.request_timeout,
            header_read_timeout: app.header_read_timeout,
            idle_timeout: app.idle_timeout,
            drain_timeout: app.drain_timeout,
            keep_alive: app.keep_alive,
            listen_addr: app.listen_addr,
            trust_proxy: app.trust_proxy,
            reuseport: app.reuseport,
            hsts: app.hsts,
            alt_svc: app.alt_svc.clone(),
        }
    }
}

pub(crate) struct AppInner {
    pub(crate) compiled: Arc<CompiledRouter>,
    pub(crate) max_body_size: usize,
    pub(crate) max_connections: usize,
    pub(crate) max_upgraded: UpgradeBudget,
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
    pub(crate) hsts: bool,
    pub(crate) alt_svc: Option<String>,
    pub(crate) route_count: usize,
    pub(crate) explain: String,
}

impl AppInner {
    fn from_settings(
        compiled: Arc<CompiledRouter>,
        route_count: usize,
        explain: String,
        s: AppSettings,
    ) -> Self {
        Self {
            compiled,
            max_body_size: s.max_body_size,
            max_connections: s.max_connections,
            max_upgraded: UpgradeBudget(Arc::new(Semaphore::new(s.max_upgraded_connections))),
            max_concurrent_streams: s.max_concurrent_streams,
            max_headers: s.max_headers,
            max_buf_size: s.max_buf_size,
            request_timeout: s.request_timeout,
            header_read_timeout: s.header_read_timeout,
            idle_timeout: s.idle_timeout,
            drain_timeout: s.drain_timeout,
            keep_alive: s.keep_alive,
            listen_addr: s.listen_addr,
            trust_proxy: s.trust_proxy,
            reuseport: s.reuseport,
            hsts: s.hsts,
            alt_svc: s.alt_svc,
            route_count,
            explain,
        }
    }

    pub(crate) async fn handle(&self, req: Request) -> Response {
        self.compiled.dispatch(req).await
    }

    pub(crate) fn state(&self) -> Arc<StateMap> {
        Arc::clone(&self.compiled.state)
    }

    pub(crate) fn conn_header_timeout(&self) -> Duration {
        self.header_read_timeout.min(self.idle_timeout)
    }
}
