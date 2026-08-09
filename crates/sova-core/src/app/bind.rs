//! Unified bind target for [`super::BoundApp::serve`].

use crate::error::{Error, Result};
use crate::server;
use crate::App;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;

/// Where to accept connections.
pub enum Bind {
    Port(u16),
    Addr(SocketAddr),
    Str(String),
    Env {
        default_port: u16,
    },
    Listener(std::net::TcpListener),
    #[cfg(unix)]
    Uds(PathBuf),
}

impl From<u16> for Bind {
    fn from(port: u16) -> Self {
        Self::Port(port)
    }
}

impl From<SocketAddr> for Bind {
    fn from(addr: SocketAddr) -> Self {
        Self::Addr(addr)
    }
}

impl From<&str> for Bind {
    fn from(s: &str) -> Self {
        Self::Str(s.to_string())
    }
}

impl From<String> for Bind {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}

/// HTTP protocol mode for a bound app.
///
/// `all` is preparation for HTTP/3 discovery: TCP responses advertise `Alt-Svc`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http {
    H1,
    H1H2,
    All,
}

impl Http {
    pub const fn h1() -> Self {
        Self::H1
    }
    pub const fn h1_h2() -> Self {
        Self::H1H2
    }
    pub const fn all() -> Self {
        Self::All
    }
}

impl Bind {
    pub fn str(s: impl Into<String>) -> Self {
        Self::Str(s.into())
    }
}

/// App + bind target + optional programmatic shutdown.
pub struct BoundApp {
    app: App,
    bind: Bind,
    http: Http,
    shutdown: Option<server::ExternalShutdown>,
    #[cfg(feature = "tls")]
    tls: Option<crate::tls::TlsRuntime>,
}

impl App {
    /// Choose a bind target; call [`.serve()`](BoundApp::serve) (optionally after [`.shutdown(...)`](BoundApp::shutdown)).
    pub fn bind(self, target: impl Into<Bind>) -> BoundApp {
        BoundApp {
            app: self,
            bind: target.into(),
            http: Http::h1_h2(),
            shutdown: None,
            #[cfg(feature = "tls")]
            tls: None,
        }
    }

    /// Bind `0.0.0.0:port`, run CLI if present, otherwise serve.
    ///
    /// Prefer this for the common case; use [`Self::bind`] for custom addresses.
    /// When a plugin attached TLS via [`Self::use_tls`] (e.g. Acme), this serves HTTPS.
    pub async fn listen(self, port: u16) -> Result<()> {
        self.bind(port).run().await
    }

    /// Attach TLS for the next [`Self::listen`] / [`BoundApp::run`].
    ///
    /// Plugins like Acme call this during `install` so apps do not need a separate
    /// `.tls(...)` on [`BoundApp`]. An explicit [`BoundApp::tls`] still wins.
    #[cfg(feature = "tls")]
    pub fn use_tls(&mut self, tls: crate::Tls) -> &mut Self {
        self.tls = Some(tls);
        self
    }
}

impl BoundApp {
    /// Select HTTP protocol mode.
    ///
    /// `Http::all()` enables automatic `Alt-Svc: h3=":<port>"; ma=86400`.
    pub fn http(mut self, http: Http) -> Self {
        self.http = http;
        self
    }

    /// Enable `SO_REUSEPORT` on TCP bind (requires feature `listen-reuseport`).
    pub fn reuseport(mut self, enabled: bool) -> Self {
        self.app.reuseport = enabled;
        self
    }

    /// CLI commands then [`Self::serve`]. Same built-in commands as [`App::run`].
    pub async fn run(self) -> Result<()> {
        crate::tracing_init::ensure_tracing();
        let args: Vec<String> = std::env::args().skip(1).collect();
        if self.app.run_cli_command(&args).await? {
            return Ok(());
        }
        self.serve_inner().await
    }

    fn apply_http_mode(mut app: App, http: Http, port: Option<u16>) -> App {
        match http {
            Http::All => {
                if let Some(port) = port {
                    app.alt_svc = Some(format!("h3=\":{port}\"; ma=86400"));
                }
            }
            _ => {
                app.alt_svc = None;
            }
        }
        app
    }

    /// Stop when this future completes (in addition to Ctrl-C / SIGTERM).
    pub fn shutdown<F>(mut self, f: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.shutdown = Some(Box::pin(f));
        self
    }

    /// Enable HTTPS (TCP binds only). Requires feature `tls`.
    ///
    /// Overrides TLS previously attached with [`App::use_tls`].
    #[cfg(feature = "tls")]
    pub fn tls(mut self, config: crate::Tls) -> Result<Self> {
        if matches!(self.bind, Bind::Uds(_)) {
            return Err(Error::Internal(
                "TLS cannot be combined with Bind::Uds".into(),
            ));
        }
        self.app.tls = None;
        self.tls = Some(config.into_runtime()?);
        self.app.hsts = self.tls.as_ref().map(|t| t.hsts).unwrap_or(false);
        Ok(self)
    }

    /// Serve HTTP (no CLI). Prefer [`Self::run`] from `main` so `check`/`routes` work.
    pub async fn serve(self) -> Result<()> {
        crate::tracing_init::ensure_tracing();
        self.serve_inner().await
    }

    #[cfg(feature = "tls")]
    fn take_tls_runtime(&mut self) -> Result<Option<crate::tls::TlsRuntime>> {
        if self.tls.is_some() {
            return Ok(self.tls.take());
        }
        let Some(config) = self.app.tls.take() else {
            return Ok(None);
        };
        if matches!(self.bind, Bind::Uds(_)) {
            return Err(Error::Internal(
                "TLS cannot be combined with Bind::Uds".into(),
            ));
        }
        let runtime = config.into_runtime()?;
        self.app.hsts = runtime.hsts;
        Ok(Some(runtime))
    }

    #[cfg(feature = "tls")]
    async fn serve_inner(mut self) -> Result<()> {
        let tls = self.take_tls_runtime()?;
        let http = self.http;
        match self.bind {
            Bind::Port(port) => {
                let app = Self::apply_http_mode(self.app, http, Some(port));
                server::listen(app, Some(port), None, self.shutdown, tls).await
            }
            Bind::Addr(addr) => {
                let app = Self::apply_http_mode(self.app, http, Some(addr.port()));
                server::listen(app, None, Some(addr), self.shutdown, tls).await
            }
            Bind::Str(s) => {
                let addr: SocketAddr = s.parse().map_err(|e| {
                    Error::Internal(format!("bind str {s:?}: invalid address: {e}"))
                })?;
                let app = Self::apply_http_mode(self.app, http, Some(addr.port()));
                server::listen(app, None, Some(addr), self.shutdown, tls).await
            }
            Bind::Env { default_port } => {
                let addr = super::addr_from_env(default_port)?;
                let app = Self::apply_http_mode(self.app, http, Some(addr.port()));
                server::listen(app, None, Some(addr), self.shutdown, tls).await
            }
            Bind::Listener(listener) => {
                let port = listener.local_addr().ok().map(|a| a.port());
                let app = Self::apply_http_mode(self.app, http, port);
                server::listen_with_listener(app, listener, self.shutdown, tls).await
            }
            #[cfg(unix)]
            Bind::Uds(path) => {
                let app = Self::apply_http_mode(self.app, http, None);
                server::listen_uds(app, &path, self.shutdown).await
            }
        }
    }

    #[cfg(not(feature = "tls"))]
    async fn serve_inner(self) -> Result<()> {
        let http = self.http;
        match self.bind {
            Bind::Port(port) => {
                let app = Self::apply_http_mode(self.app, http, Some(port));
                server::listen(app, Some(port), None, self.shutdown).await
            }
            Bind::Addr(addr) => {
                let app = Self::apply_http_mode(self.app, http, Some(addr.port()));
                server::listen(app, None, Some(addr), self.shutdown).await
            }
            Bind::Str(s) => {
                let addr: SocketAddr = s.parse().map_err(|e| {
                    Error::Internal(format!("bind str {s:?}: invalid address: {e}"))
                })?;
                let app = Self::apply_http_mode(self.app, http, Some(addr.port()));
                server::listen(app, None, Some(addr), self.shutdown).await
            }
            Bind::Env { default_port } => {
                let addr = super::addr_from_env(default_port)?;
                let app = Self::apply_http_mode(self.app, http, Some(addr.port()));
                server::listen(app, None, Some(addr), self.shutdown).await
            }
            Bind::Listener(listener) => {
                let port = listener.local_addr().ok().map(|a| a.port());
                let app = Self::apply_http_mode(self.app, http, port);
                server::listen_with_listener(app, listener, self.shutdown).await
            }
            #[cfg(unix)]
            Bind::Uds(path) => {
                let app = Self::apply_http_mode(self.app, http, None);
                server::listen_uds(app, &path, self.shutdown).await
            }
        }
    }
}
