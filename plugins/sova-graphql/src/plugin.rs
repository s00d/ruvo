//! Plugin: outbound client (primary) + optional schema mount.

use crate::client::GraphQlClient;
use crate::fake::FakeGraphql;
use sova_core::{App, Plugin};

#[cfg(feature = "server")]
use crate::server::{
    default_graphiql_path, default_subscriptions_path, install_server, SchemaHandle,
    ServerMountConfig,
};
#[cfg(feature = "server")]
use async_graphql::{ObjectType, Schema, SubscriptionType};

enum Mode {
    Client {
        endpoint: String,
    },
    Fake {
        fake: FakeGraphql,
        endpoint: String,
    },
    #[cfg(feature = "server")]
    Server {
        handle: SchemaHandle,
    },
}

/// Optional outbound client installed together with server mount.
enum Outbound {
    Http(String),
    FromEnv,
}

/// GraphQL plugin. Prefer [`GraphQl::client`] / [`GraphQl::fake`]; server mount is optional.
pub struct GraphQl {
    mode: Mode,
    path: String,
    path_explicit: bool,
    graphiql: bool,
    graphiql_explicit: bool,
    enabled: bool,
    #[cfg(feature = "server")]
    outbound: Option<Outbound>,
    #[cfg(feature = "server")]
    graphiql_path: Option<String>,
    #[cfg(feature = "server")]
    graphiql_path_explicit: bool,
    #[cfg(feature = "server")]
    subscriptions_path: Option<String>,
    #[cfg(feature = "server")]
    subscriptions_disabled: bool,
    #[cfg(feature = "server")]
    allow_get_queries: bool,
    #[cfg(feature = "server")]
    sdl_path: Option<String>,
    #[cfg(feature = "server")]
    sdl_path_explicit: bool,
}

impl GraphQl {
    fn server_defaults() -> Self {
        Self {
            mode: Mode::Client {
                endpoint: String::new(),
            },
            path: "/graphql".into(),
            path_explicit: false,
            graphiql: cfg!(debug_assertions),
            graphiql_explicit: false,
            enabled: true,
            #[cfg(feature = "server")]
            outbound: None,
            #[cfg(feature = "server")]
            graphiql_path: None,
            #[cfg(feature = "server")]
            graphiql_path_explicit: false,
            #[cfg(feature = "server")]
            subscriptions_path: None,
            #[cfg(feature = "server")]
            subscriptions_disabled: false,
            #[cfg(feature = "server")]
            allow_get_queries: false,
            #[cfg(feature = "server")]
            sdl_path: None,
            #[cfg(feature = "server")]
            sdl_path_explicit: false,
        }
    }

    /// Outbound client against a remote GraphQL HTTP endpoint.
    pub fn client(endpoint: impl Into<String>) -> Self {
        Self {
            mode: Mode::Client {
                endpoint: endpoint.into(),
            },
            path: "/graphql".into(),
            path_explicit: false,
            graphiql: false,
            graphiql_explicit: true,
            enabled: true,
            #[cfg(feature = "server")]
            outbound: None,
            #[cfg(feature = "server")]
            graphiql_path: None,
            #[cfg(feature = "server")]
            graphiql_path_explicit: false,
            #[cfg(feature = "server")]
            subscriptions_path: None,
            #[cfg(feature = "server")]
            subscriptions_disabled: true,
            #[cfg(feature = "server")]
            allow_get_queries: false,
            #[cfg(feature = "server")]
            sdl_path: None,
            #[cfg(feature = "server")]
            sdl_path_explicit: false,
        }
    }

    /// Outbound client with in-memory stubs (tests).
    pub fn fake(fake: FakeGraphql) -> Self {
        Self {
            mode: Mode::Fake {
                fake,
                endpoint: "fake://graphql".into(),
            },
            path: "/graphql".into(),
            path_explicit: false,
            graphiql: false,
            graphiql_explicit: true,
            enabled: true,
            #[cfg(feature = "server")]
            outbound: None,
            #[cfg(feature = "server")]
            graphiql_path: None,
            #[cfg(feature = "server")]
            graphiql_path_explicit: false,
            #[cfg(feature = "server")]
            subscriptions_path: None,
            #[cfg(feature = "server")]
            subscriptions_disabled: true,
            #[cfg(feature = "server")]
            allow_get_queries: false,
            #[cfg(feature = "server")]
            sdl_path: None,
            #[cfg(feature = "server")]
            sdl_path_explicit: false,
        }
    }

    /// Mount an `async-graphql` schema (requires feature `server`).
    #[cfg(feature = "server")]
    pub fn server<Q, M, S>(schema: Schema<Q, M, S>) -> Self
    where
        Q: ObjectType + 'static,
        M: ObjectType + 'static,
        S: SubscriptionType + 'static,
    {
        let mut this = Self::server_defaults();
        this.mode = Mode::Server {
            handle: SchemaHandle::from_schema(schema),
        };
        this
    }

    /// Also install an outbound client (BFF: mount + remote GraphQL).
    #[cfg(feature = "server")]
    pub fn with_client(mut self, endpoint: impl Into<String>) -> Self {
        self.outbound = Some(Outbound::Http(endpoint.into()));
        self
    }

    /// Outbound client URL from `GRAPHQL_URL` (with server mount).
    #[cfg(feature = "server")]
    pub fn with_client_from_env(mut self) -> Self {
        self.outbound = Some(Outbound::FromEnv);
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self.path_explicit = true;
        self
    }

    pub fn graphiql(mut self, enabled: bool) -> Self {
        self.graphiql = enabled;
        self.graphiql_explicit = true;
        self
    }

    /// GraphiQL UI path (default `/graphiql` when enabled).
    #[cfg(feature = "server")]
    pub fn graphiql_path(mut self, path: impl Into<String>) -> Self {
        self.graphiql_path = Some(path.into());
        self.graphiql_path_explicit = true;
        self
    }

    /// WebSocket endpoint for subscriptions (default `{api_path}/ws`).
    #[cfg(feature = "server")]
    pub fn subscriptions(mut self, path: impl Into<String>) -> Self {
        self.subscriptions_path = Some(path.into());
        self.subscriptions_disabled = false;
        self
    }

    /// Disable subscription WebSocket mount.
    #[cfg(feature = "server")]
    pub fn without_subscriptions(mut self) -> Self {
        self.subscriptions_disabled = true;
        self
    }

    /// Allow GraphQL queries over HTTP GET on the API path (`?query=`).
    #[cfg(feature = "server")]
    pub fn allow_get_queries(mut self, enabled: bool) -> Self {
        self.allow_get_queries = enabled;
        self
    }

    /// Expose schema SDL at GET path (e.g. `/graphql/sdl`).
    #[cfg(feature = "server")]
    pub fn sdl_path(mut self, path: impl Into<String>) -> Self {
        self.sdl_path = Some(path.into());
        self.sdl_path_explicit = true;
        self
    }

    #[cfg(feature = "server")]
    fn install_outbound(&self, app: &mut App) {
        let Some(outbound) = &self.outbound else {
            return;
        };
        let endpoint = match outbound {
            Outbound::Http(url) => url.clone(),
            Outbound::FromEnv => std::env::var("GRAPHQL_URL").unwrap_or_default(),
        };
        if !endpoint.is_empty() {
            app.state(GraphQlClient::http(endpoint));
        }
    }

    #[cfg(feature = "server")]
    fn server_mount_config(&self) -> ServerMountConfig {
        let graphiql_path = if self.graphiql_path_explicit {
            self.graphiql_path
                .clone()
                .unwrap_or_else(|| default_graphiql_path(&self.path))
        } else {
            default_graphiql_path(&self.path)
        };
        let subscriptions_path = if self.subscriptions_disabled {
            None
        } else if let Some(p) = &self.subscriptions_path {
            Some(p.clone())
        } else {
            Some(default_subscriptions_path(&self.path))
        };
        let sdl_path = if self.sdl_path_explicit {
            self.sdl_path.clone()
        } else {
            None
        };
        ServerMountConfig {
            path: self.path.clone(),
            graphiql: self.graphiql,
            graphiql_path,
            subscriptions_path,
            allow_get_queries: self.allow_get_queries,
            sdl_path,
        }
    }
}

impl Plugin for GraphQl {
    fn id(&self) -> &'static str {
        "graphql"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("GraphQL")
            .description("Outbound GraphQL client (+ optional schema mount)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("graphql") {
                if let Some(en) = section.get("enabled").and_then(|v| v.as_bool()) {
                    self.enabled = en;
                }
                if !self.path_explicit {
                    if let Some(p) = section.get("path").and_then(|v| v.as_str()) {
                        self.path = p.to_string();
                    }
                }
                if !self.graphiql_explicit {
                    if let Some(g) = section.get("graphiql").and_then(|v| v.as_bool()) {
                        self.graphiql = g;
                    }
                }
                #[cfg(feature = "server")]
                {
                    if !self.graphiql_path_explicit {
                        if let Some(p) = section.get("graphiql_path").and_then(|v| v.as_str()) {
                            self.graphiql_path = Some(p.to_string());
                            self.graphiql_path_explicit = true;
                        }
                    }
                    if self.subscriptions_path.is_none() && !self.subscriptions_disabled {
                        if let Some(p) = section.get("subscriptions_path").and_then(|v| v.as_str())
                        {
                            self.subscriptions_path = Some(p.to_string());
                        }
                    }
                    if let Some(v) = section.get("allow_get_queries").and_then(|v| v.as_bool()) {
                        self.allow_get_queries = v;
                    }
                    if !self.sdl_path_explicit {
                        if let Some(p) = section.get("sdl_path").and_then(|v| v.as_str()) {
                            self.sdl_path = Some(p.to_string());
                            self.sdl_path_explicit = true;
                        }
                    }
                }
                if let Mode::Client { endpoint } = &mut self.mode {
                    if endpoint.is_empty() {
                        if let Some(u) = section.get("url").and_then(|v| v.as_str()) {
                            *endpoint = u.to_string();
                        }
                    }
                }
            }
        }

        if !self.enabled {
            return;
        }

        #[cfg(feature = "server")]
        if matches!(self.mode, Mode::Server { .. }) {
            self.install_outbound(app);
        }

        #[cfg(feature = "server")]
        let server_cfg = if matches!(&self.mode, Mode::Server { .. }) {
            Some(self.server_mount_config())
        } else {
            None
        };

        match self.mode {
            Mode::Client { endpoint } => {
                let endpoint = if endpoint.is_empty() {
                    std::env::var("GRAPHQL_URL").unwrap_or_default()
                } else {
                    endpoint
                };
                app.state(GraphQlClient::http(endpoint));
            }
            Mode::Fake { fake, endpoint } => {
                app.state(GraphQlClient::with_fake(endpoint, fake));
            }
            #[cfg(feature = "server")]
            Mode::Server { handle } => {
                install_server(app, handle, server_cfg.expect("server cfg"));
            }
        }
    }
}
