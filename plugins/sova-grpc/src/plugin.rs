//! Plugin: client / fake / server.

use crate::client::GrpcClient;
use crate::fake::FakeGrpc;
use crate::router::MethodRouter;
use crate::server::{mount_on_app, GrpcBindService};
use sova_core::{App, Plugin};
use std::net::SocketAddr;

enum Mode {
    Client { base: String },
    Fake { fake: FakeGrpc },
    Server {
        router: MethodRouter,
        bind: Option<SocketAddr>,
        mount: bool,
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
        }
    }
}

pub struct GrpcServerBuilder {
    router: MethodRouter,
    bind: Option<SocketAddr>,
    mount: bool,
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
                if let Mode::Server { bind, .. } = &mut self.mode {
                    if bind.is_none() {
                        if let Some(b) = section.get("bind").and_then(|v| v.as_str()) {
                            *bind = b.parse().ok();
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
                app.state(GrpcClient::http(base));
            }
            Mode::Fake { fake } => {
                app.state(GrpcClient::with_fake("fake://grpc", fake));
            }
            Mode::Server {
                router,
                bind,
                mount,
            } => {
                app.state(router.clone());
                if mount {
                    mount_on_app(app, router.clone());
                }
                if let Some(addr) = bind {
                    app.service(GrpcBindService::new(addr, router));
                }
            }
        }
    }
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
