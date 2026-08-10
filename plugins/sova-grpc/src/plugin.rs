//! Plugin: client / fake / server.

use crate::client::GrpcClient;
use crate::fake::FakeGrpc;
use crate::router::MethodRouter;
use crate::server::{mount_on_app, GrpcBindService};
use serde_json::json;
use sova_core::{App, DevToolsConfigRegistry, Plugin};
use std::net::SocketAddr;

enum Mode {
    Client {
        base: String,
    },
    Fake {
        fake: FakeGrpc,
    },
    Server {
        router: MethodRouter,
        bind: Option<SocketAddr>,
        mount: bool,
        client_base: Option<String>,
        client_from_env: bool,
    },
}

/// Connect-JSON unary RPC. Prefer [`Grpc::client`] / [`Grpc::fake`].
pub struct Grpc {
    mode: Mode,
}

impl Grpc {
    pub fn client(base: impl Into<String>) -> Self {
        Self {
            mode: Mode::Client { base: base.into() },
        }
    }

    pub fn fake(fake: FakeGrpc) -> Self {
        Self {
            mode: Mode::Fake { fake },
        }
    }

    pub fn server() -> GrpcServerBuilder {
        GrpcServerBuilder {
            router: MethodRouter::new(),
            bind: None,
            mount: true,
            client_base: None,
            client_from_env: false,
        }
    }
}

pub struct GrpcServerBuilder {
    router: MethodRouter,
    bind: Option<SocketAddr>,
    mount: bool,
    client_base: Option<String>,
    client_from_env: bool,
}

impl GrpcServerBuilder {
    pub fn unary<Req, Res, F, Fut>(self, method: impl Into<String>, f: F) -> Self
    where
        Req: serde::de::DeserializeOwned + Send + 'static,
        Res: serde::Serialize + Send + 'static,
        F: Fn(Req) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Res, crate::GrpcError>> + Send + 'static,
    {
        self.router.unary(method, f);
        self
    }

    /// Unary handler with access to the incoming HTTP [`sova_core::Request`].
    pub fn unary_with_request<Req, Res, F, Fut>(self, method: impl Into<String>, f: F) -> Self
    where
        Req: serde::de::DeserializeOwned + Send + 'static,
        Res: serde::Serialize + Send + 'static,
        F: Fn(sova_core::Request, Req) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Res, crate::GrpcError>> + Send + 'static,
    {
        self.router.unary_with_request(method, f);
        self
    }

    /// Also install an outbound client (BFF: serve + call remote RPC).
    pub fn client(mut self, base: impl Into<String>) -> Self {
        self.client_base = Some(base.into());
        self
    }

    /// Outbound client base URL from `GRPC_URL`.
    pub fn client_from_env(mut self) -> Self {
        self.client_from_env = true;
        self
    }

    /// Also listen on a dedicated socket (BackgroundService).
    pub fn bind(mut self, addr: impl Into<String>) -> Self {
        let s = addr.into();
        self.bind = s.parse().ok();
        self
    }

    /// Mount unary paths on the main HTTP app (default true).
    pub fn mount(mut self, enabled: bool) -> Self {
        self.mount = enabled;
        self
    }

    pub fn build(self) -> Grpc {
        Grpc {
            mode: Mode::Server {
                router: self.router,
                bind: self.bind,
                mount: self.mount,
                client_base: self.client_base,
                client_from_env: self.client_from_env,
            },
        }
    }
}

impl From<GrpcServerBuilder> for Grpc {
    fn from(b: GrpcServerBuilder) -> Self {
        b.build()
    }
}

impl Plugin for Grpc {
    fn id(&self) -> &'static str {
        "grpc"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("gRPC")
            .description("Connect-JSON unary RPC client (+ optional server)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("grpc") {
                if let Mode::Client { base } = &mut self.mode {
                    if base.is_empty() {
                        if let Some(u) = section.get("client_url").and_then(|v| v.as_str()) {
                            *base = u.to_string();
                        }
                    }
                }
                if let Mode::Server {
                    bind,
                    client_base,
                    client_from_env,
                    ..
                } = &mut self.mode
                {
                    if bind.is_none() {
                        if let Some(b) = section.get("bind").and_then(|v| v.as_str()) {
                            *bind = b.parse().ok();
                        }
                    }
                    if client_base.is_none() && !*client_from_env {
                        if let Some(u) = section.get("client_url").and_then(|v| v.as_str()) {
                            *client_base = Some(u.to_string());
                        }
                    }
                }
            }
        }

        match self.mode {
            Mode::Client { base } => {
                let base = if base.is_empty() {
                    std::env::var("GRPC_URL").unwrap_or_default()
                } else {
                    base
                };
                app.state(GrpcClient::http(base.clone()));
                register_devtools_mount(app, &base, &[], None);
            }
            Mode::Fake { fake } => {
                app.state(GrpcClient::with_fake("fake://grpc", fake));
                register_devtools_mount(app, "fake://grpc", &[], None);
            }
            Mode::Server {
                router,
                bind,
                mount,
                client_base,
                client_from_env,
            } => {
                let outbound = if client_from_env {
                    std::env::var("GRPC_URL").unwrap_or_default()
                } else {
                    client_base.unwrap_or_default()
                };
                if !outbound.is_empty() {
                    app.state(GrpcClient::http(outbound.clone()));
                }
                let methods = router.methods();
                app.state(router.clone());
                if mount {
                    mount_on_app(app, router.clone());
                }
                let bind_label = bind.map(|a| a.to_string());
                if let Some(addr) = bind {
                    app.service(GrpcBindService::new(addr, router));
                }
                register_devtools_mount(
                    app,
                    if outbound.is_empty() {
                        "in-process"
                    } else {
                        &outbound
                    },
                    &methods,
                    bind_label,
                );
            }
        }
    }
}

fn register_devtools_mount(
    app: &mut App,
    client_base: &str,
    methods: &[String],
    bind: Option<String>,
) {
    if app.try_state::<DevToolsConfigRegistry>().is_none() {
        app.state(DevToolsConfigRegistry::default());
    }
    let reg = app
        .try_state::<DevToolsConfigRegistry>()
        .expect("DevToolsConfigRegistry");
    reg.set(
        "grpc",
        json!({
            "client_base": client_base,
            "methods": methods,
            "bind": bind,
        }),
    );
}

// Allow `app.install(Grpc::server().unary(...))` without `.build()`
impl Plugin for GrpcServerBuilder {
    fn id(&self) -> &'static str {
        "grpc"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("gRPC")
            .description("Connect-JSON unary RPC client (+ optional server)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        Grpc::from(self).install(app);
    }
}
