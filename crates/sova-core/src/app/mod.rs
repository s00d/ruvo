mod bind;
mod hooks;

pub(crate) use hooks::{AppInner, ListenParts, ShutdownHook, StartupHook};
pub use bind::{Bind, BoundApp, Http};
pub use hooks::Server;

use crate::error::{Error, Result};
use crate::handler::BoxFuture;
use crate::plugin::{
    check_plugin_sdk, InstalledPlugin, Plugin, SdkCompat, PLUGIN_SDK_VERSION,
};
use crate::request::Request;
use crate::response::Response;
use crate::router::Router;
use crate::service::{BackgroundService, BoxedService};
use crate::state::StateMap;
use bytes::Bytes;
use http::Method;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

pub(crate) type CliCommandFn =
    Arc<dyn Fn(Arc<StateMap>, Vec<String>) -> BoxFuture<Result<()>> + Send + Sync>;

pub(crate) type CheckFn =
    Arc<dyn Fn(Arc<StateMap>) -> BoxFuture<Result<()>> + Send + Sync>;

/// Kind of app check — readiness probes vs deploy-time audits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckKind {
    /// Runtime dependencies (db, redis, storage, …) — used by `GET /ready`.
    Ready,
    /// Deploy-time / config audits (openapi, vld, templates, …) — CLI `check` only.
    Audit,
}

/// Outcome of one named check from [`App::run_checks`].
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: &'static str,
    pub ok: bool,
    pub error: Option<String>,
}

type CheckEntry = (&'static str, CheckKind, CheckFn);
type CheckList = Arc<std::sync::Mutex<Vec<CheckEntry>>>;

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
    pub(crate) trust_proxy: bool,
    pub(crate) reuseport: bool,
    /// Set by CLI listen helpers — BackgroundServices skipped unless `service_in_cli`.
    pub(crate) cli_mode: bool,
    pub(crate) service_in_cli: bool,
    pub(crate) hsts: bool,
    pub(crate) alt_svc: Option<String>,
    pub(crate) installed_plugins: HashSet<&'static str>,
    pub(crate) installed_plugin_meta: Vec<InstalledPlugin>,
    pub(crate) missing_plugin_requires: Vec<(&'static str, &'static str)>,
    pub(crate) duplicate_plugin_ids: Vec<&'static str>,
    pub(crate) plugin_sdk_errors: Vec<String>,
    pub(crate) on_startup: Vec<StartupHook>,
    pub(crate) on_shutdown: Vec<ShutdownHook>,
    pub(crate) services: Vec<BoxedService>,
    pub(crate) cli_commands: HashMap<&'static str, CliCommandFn>,
    pub(crate) checks: CheckList,
    pub(crate) probes: bool,
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
            trust_proxy: false,
            reuseport: false,
            cli_mode: false,
            service_in_cli: false,
            hsts: false,
            alt_svc: None,
            installed_plugins: HashSet::new(),
            installed_plugin_meta: Vec::new(),
            missing_plugin_requires: Vec::new(),
            duplicate_plugin_ids: Vec::new(),
            plugin_sdk_errors: Vec::new(),
            on_startup: Vec::new(),
            on_shutdown: Vec::new(),
            services: Vec::new(),
            cli_commands: HashMap::new(),
            checks: Arc::new(std::sync::Mutex::new(Vec::new())),
            probes: false,
        }
    }

    /// Register a plugin CLI command handled by [`Self::run`] (e.g. `"migrate"`).
    pub fn register_cli<F, Fut>(&mut self, name: &'static str, f: F) -> &mut Self
    where
        F: Fn(Arc<StateMap>, Vec<String>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.cli_commands
            .insert(name, Arc::new(move |state, args| Box::pin(f(state, args))));
        self
    }

    /// Register a readiness check (`CheckKind::Ready`) for `GET /ready` and CLI `check`.
    pub fn register_check<F, Fut>(&mut self, name: &'static str, f: F) -> &mut Self
    where
        F: Fn(Arc<StateMap>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.push_check(name, CheckKind::Ready, f)
    }

    /// Register a deploy-time audit (`CheckKind::Audit`) — CLI `check` only, not `/ready`.
    pub fn register_audit<F, Fut>(&mut self, name: &'static str, f: F) -> &mut Self
    where
        F: Fn(Arc<StateMap>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.push_check(name, CheckKind::Audit, f)
    }

    fn push_check<F, Fut>(&mut self, name: &'static str, kind: CheckKind, f: F) -> &mut Self
    where
        F: Fn(Arc<StateMap>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.checks
            .lock()
            .expect("checks lock")
            .push((name, kind, Arc::new(move |state| Box::pin(f(state)))));
        self
    }

    /// Run registered checks filtered by [`CheckKind`].
    pub async fn run_checks(
        &self,
        state: Arc<StateMap>,
        kinds: &[CheckKind],
    ) -> Vec<CheckResult> {
        run_check_list(&self.checks, state, kinds).await
    }

    /// Install k8s-style probes: `GET /healthz` (liveness) and `GET /ready` (Ready checks).
    ///
    /// Idempotent. Presets (`App::web` / `App::api`) call this automatically.
    pub fn with_probes(&mut self) -> &mut Self {
        if self.probes {
            return self;
        }
        self.probes = true;
        let checks = Arc::clone(&self.checks);

        self.get("/healthz", || async {
            Response::json(&serde_json::json!({ "status": "ok" }))
        });

        self.get("/ready", move |req: Request| {
            let checks = Arc::clone(&checks);
            async move {
                let results =
                    run_check_list(&checks, req.states(), &[CheckKind::Ready]).await;
                ready_response(&results)
            }
        });

        self
    }

    pub fn max_body_size(&mut self, bytes: usize) -> &mut Self {
        self.max_body_size = bytes;
        self.router.defaults.insert(crate::limits::MaxBody::bytes(bytes));
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
        if let Some(d) = timeout {
            self.router
                .defaults
                .insert(crate::limits::RequestTimeout(d));
        }
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

    /// When true, `ClientAddr` may use `X-Forwarded-For` / `Forwarded` (only behind a trusted proxy).
    pub fn trust_proxy(&mut self, trust: bool) -> &mut Self {
        self.trust_proxy = trust;
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
        let plugin_id = plugin.id();
        if self.installed_plugins.contains(plugin_id) {
            self.duplicate_plugin_ids.push(plugin_id);
            return self;
        }
        let meta = plugin.meta();
        for dep in plugin.requires() {
            if !self.installed_plugins.contains(dep) {
                self.missing_plugin_requires.push((plugin_id, dep));
            }
        }
        match check_plugin_sdk(meta.sdk, PLUGIN_SDK_VERSION) {
            SdkCompat::Ok => {}
            SdkCompat::Warn { core, plugin: declared } => {
                tracing::warn!(
                    plugin = plugin_id,
                    plugin_sdk = %declared,
                    core_sdk = %core,
                    "plugin SDK is older than core; consider rebuilding against the current Plugin SDK"
                );
            }
            SdkCompat::Error(msg) => {
                self.plugin_sdk_errors
                    .push(format!("plugin `{plugin_id}`: {msg}"));
            }
        }
        self.installed_plugin_meta.push(InstalledPlugin {
            id: plugin_id,
            meta,
        });
        plugin.install(self);
        self.installed_plugins.insert(plugin_id);
        self
    }

    /// Whether a plugin with this [`Plugin::id`] was already installed.
    pub fn has_plugin(&self, id: &str) -> bool {
        self.installed_plugins.contains(id)
    }

    /// Metadata for every plugin passed to [`Self::install`] (order preserved).
    pub fn installed_plugin_meta(&self) -> &[InstalledPlugin] {
        &self.installed_plugin_meta
    }

    /// Primary app entrypoint: run as a server process.
    ///
    /// CLI mode:
    /// - `check`
    /// - `routes`
    /// - `plugins`
    /// - `openapi --out <path>`
    /// - `tasks`
    /// - `i18n missing`
    ///
    /// Non-CLI mode binds via [`Bind::Env`] (`HOST`/`PORT`, default port `3000`).
    /// Prefer [`App::bind`] + [`BoundApp::run`] when the address is fixed in code.
    pub async fn run(self) -> Result<()> {
        self.bind(Bind::Env { default_port: 3000 }).run().await
    }

    pub(crate) async fn run_cli_command(&self, args: &[String]) -> Result<bool> {
        let Some(cmd) = args.first().map(String::as_str) else {
            return Ok(false);
        };

        let server = self.build()?;
        let state = server.state();
        for hook in &server.startups {
            hook(Arc::clone(&state)).await?;
        }

        let rest: Vec<String> = args.iter().skip(1).cloned().collect();
        let handled = if let Some(handler) = self.cli_commands.get(cmd) {
            handler(Arc::clone(&state), rest).await?;
            true
        } else {
            match cmd {
                "check" => {
                    // `build()` already ran — plugin `requires()` are satisfied.
                    println!("ok plugins");
                    let results = self
                        .run_checks(
                            Arc::clone(&state),
                            &[CheckKind::Ready, CheckKind::Audit],
                        )
                        .await;
                    let mut failed = false;
                    for r in &results {
                        if r.ok {
                            println!("ok {}", r.name);
                        } else {
                            println!(
                                "fail {} — {}",
                                r.name,
                                r.error.as_deref().unwrap_or("")
                            );
                            failed = true;
                        }
                    }
                    if failed {
                        return Err(Error::Internal(
                            "one or more checks failed".into(),
                        ));
                    }
                    println!("ok");
                    true
                }
                "routes" => {
                    println!("{}", self.explain());
                    true
                }
                "plugins" => {
                    for p in &self.installed_plugin_meta {
                        let desc = if p.meta.description.is_empty() {
                            "-"
                        } else {
                            p.meta.description
                        };
                        println!(
                            "{:<24} {:<20} sdk={}  {}",
                            p.id, p.meta.name, p.meta.sdk, desc
                        );
                    }
                    if self.installed_plugin_meta.is_empty() {
                        println!("(no plugins installed)");
                    }
                    true
                }
                "openapi" => {
                    let out_idx = args.iter().position(|a| a == "--out");
                    let out_path = out_idx
                        .and_then(|idx| args.get(idx + 1))
                        .ok_or_else(|| Error::Internal("openapi requires --out <path>".into()))?;
                    let res = server
                        .handle_request(Method::GET, "/docs/openapi.json", "")
                        .await;
                    if !res.status_code().is_success() {
                        return Err(Error::Internal(format!(
                            "openapi endpoint failed with status {}",
                            res.status_code()
                        )));
                    }
                    let bytes = res
                        .body_bytes()
                        .ok_or_else(|| Error::Internal("openapi body is streaming".into()))?;
                    fs::write(out_path, bytes).map_err(|e| {
                        Error::Internal(format!("failed writing openapi to {out_path}: {e}"))
                    })?;
                    println!("wrote {}", out_path);
                    true
                }
                "tasks" => {
                    println!(
                        "tasks CLI requires the Tasks plugin (`app.install(Tasks::…)`).\n\
                         Then: tasks list | tasks schedule | tasks run NAME"
                    );
                    true
                }
                "i18n" if args.get(1).map(String::as_str) == Some("missing") => {
                    let res = server
                        .handle_request(Method::GET, "/_i18n/_missing.json", "")
                        .await;
                    if let Some(body) = res.body_bytes() {
                        println!("{}", String::from_utf8_lossy(body));
                    } else {
                        println!("i18n missing endpoint returned streaming body");
                    }
                    true
                }
                "i18n" => false,
                _ => false,
            }
        };

        if handled {
            for hook in &server.shutdowns {
                hook().await;
            }
        }
        Ok(handled)
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

async fn run_check_list(
    checks: &CheckList,
    state: Arc<StateMap>,
    kinds: &[CheckKind],
) -> Vec<CheckResult> {
    let entries: Vec<CheckEntry> = checks.lock().expect("checks lock").clone();
    let mut out = Vec::with_capacity(entries.len());
    for (name, kind, check) in entries {
        if !kinds.contains(&kind) {
            continue;
        }
        match check(Arc::clone(&state)).await {
            Ok(()) => out.push(CheckResult {
                name,
                ok: true,
                error: None,
            }),
            Err(e) => out.push(CheckResult {
                name,
                ok: false,
                error: Some(e.to_string()),
            }),
        }
    }
    out
}

fn ready_response(results: &[CheckResult]) -> Response {
    let mut checks = serde_json::Map::new();
    let mut failed = Vec::new();
    for r in results {
        if r.ok {
            checks.insert(r.name.to_string(), serde_json::json!("ok"));
        } else {
            let msg = r.error.clone().unwrap_or_else(|| "failed".into());
            checks.insert(r.name.to_string(), serde_json::json!(msg));
            failed.push(r.name);
        }
    }
    let mut res = if failed.is_empty() {
        Response::json(&serde_json::json!({
            "status": "ok",
            "checks": checks,
        }))
    } else {
        Response::json(&serde_json::json!({
            "status": "not_ready",
            "failed": failed,
            "checks": checks,
        }))
        .status(503)
    };
    // Set by `cargo sovax dev --graceful` so the orchestrator can detect the new process
    // while the old one still answers on the same REUSEPORT socket.
    if let Ok(id) = std::env::var("SOVA_INSTANCE_ID") {
        if !id.is_empty() {
            res = res.header("x-sova-instance", id);
        }
    }
    res
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
