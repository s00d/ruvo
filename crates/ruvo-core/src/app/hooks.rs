use super::App;
use crate::error::Result;
use crate::handler::BoxFuture;
use crate::request::Request;
use crate::response::Response;
use crate::router::{compile_router, CompiledRouter};
use crate::state::StateMap;
use bytes::Bytes;
use http::Method;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

pub(crate) type StartupHook = Box<dyn FnOnce(Arc<StateMap>) -> BoxFuture<Result<()>> + Send>;
pub(crate) type ShutdownHook = Box<dyn FnOnce() -> BoxFuture<()> + Send>;

/// Compiled app ready to handle requests without recompiling the router.
#[derive(Clone)]
pub struct Server {
    pub(crate) inner: Arc<AppInner>,
}

impl Server {
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

impl App {
    /// Compile routes once into a [`Server`]. Prefer this over repeated [`App::handle`].
    pub fn build(&self) -> Result<Server> {
        let router = self.router.clone_for_compile();
        let explain = router.explain();
        let route_count = router.route_entries().len();
        let compiled = Arc::new(compile_router(router)?);
        Ok(Server {
            inner: Arc::new(AppInner {
                compiled,
                max_body_size: self.max_body_size,
                max_connections: self.max_connections,
                max_headers: self.max_headers,
                max_buf_size: self.max_buf_size,
                request_timeout: self.request_timeout,
                header_read_timeout: self.header_read_timeout,
                idle_timeout: self.idle_timeout,
                drain_timeout: self.drain_timeout,
                keep_alive: self.keep_alive,
                listen_addr: self.listen_addr,
                trust_proxy: self.trust_proxy,
                route_count,
                explain,
            }),
        })
    }

    pub(crate) fn into_listen_parts(self) -> Result<(AppInner, Vec<StartupHook>, Vec<ShutdownHook>)> {
        let App {
            router,
            max_body_size,
            max_connections,
            max_headers,
            max_buf_size,
            request_timeout,
            header_read_timeout,
            idle_timeout,
            drain_timeout,
            keep_alive,
            listen_addr,
            trust_proxy,
            on_startup,
            on_shutdown,
        } = self;
        let explain = router.explain();
        let route_count = router.route_entries().len();
        let compiled = Arc::new(compile_router(router)?);
        Ok((
            AppInner {
                compiled,
                max_body_size,
                max_connections,
                max_headers,
                max_buf_size,
                request_timeout,
                header_read_timeout,
                idle_timeout,
                drain_timeout,
                keep_alive,
                listen_addr,
                trust_proxy,
                route_count,
                explain,
            },
            on_startup,
            on_shutdown,
        ))
    }
}

pub(crate) struct AppInner {
    pub(crate) compiled: Arc<CompiledRouter>,
    pub(crate) max_body_size: usize,
    pub(crate) max_connections: usize,
    pub(crate) max_headers: usize,
    pub(crate) max_buf_size: Option<usize>,
    pub(crate) request_timeout: Option<Duration>,
    pub(crate) header_read_timeout: Duration,
    pub(crate) idle_timeout: Duration,
    pub(crate) drain_timeout: Duration,
    pub(crate) keep_alive: bool,
    pub(crate) listen_addr: Option<SocketAddr>,
    pub(crate) trust_proxy: bool,
    pub(crate) route_count: usize,
    pub(crate) explain: String,
}

impl AppInner {
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
