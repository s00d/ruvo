//! Plugin: outbound client (primary) + optional schema mount.

use crate::client::GraphQlClient;
use crate::fake::FakeGraphql;
use sova_core::{App, Plugin};

#[cfg(feature = "server")]
use crate::server::SchemaHandle;
#[cfg(feature = "server")]
use async_graphql::{ObjectType, Schema, SubscriptionType};

enum Mode {
    Client { endpoint: String },
    Fake { fake: FakeGraphql, endpoint: String },
    #[cfg(feature = "server")]
    Server { handle: SchemaHandle },
}

/// GraphQL plugin. Prefer [`GraphQl::client`] / [`GraphQl::fake`]; server mount is optional.
pub struct GraphQl {
    mode: Mode,
    path: String,
    path_explicit: bool,
    graphiql: bool,
    graphiql_explicit: bool,
    enabled: bool,
}

impl GraphQl {
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
        Self {
            mode: Mode::Server {
                handle: SchemaHandle::from_schema(schema),
            },
            path: "/graphql".into(),
            path_explicit: false,
            graphiql: cfg!(debug_assertions),
            graphiql_explicit: false,
            enabled: true,
        }
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
                // Client URL from toml when using empty/from-env style later.
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
                crate::server::install_mount(app, handle, &self.path, self.graphiql);
            }
        }
    }
}
