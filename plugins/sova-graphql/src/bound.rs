//! `req.graphql()` — outbound client + optional mounted schema.

use crate::client::{GraphQlClient, PendingGraphql};
use crate::error::GraphqlError;
use sova_core::Request;

#[cfg(feature = "server")]
use crate::server::SchemaHandle;

pub trait GraphQlExt {
    fn graphql(&self) -> GraphQlBound;
    fn try_graphql(&self) -> Option<GraphQlBound>;
}

impl GraphQlExt for Request {
    fn graphql(&self) -> GraphQlBound {
        GraphQlBound {
            client: self.state::<GraphQlClient>(),
        }
    }

    fn try_graphql(&self) -> Option<GraphQlBound> {
        self.try_state::<GraphQlClient>()
            .map(|client| GraphQlBound { client })
    }
}

/// Access the mounted GraphQL schema from HTTP handlers (server install).
#[cfg(feature = "server")]
pub trait GraphqlServerExt {
    fn graphql_schema(&self) -> std::sync::Arc<SchemaHandle>;
    fn try_graphql_schema(&self) -> Option<std::sync::Arc<SchemaHandle>>;
}

#[cfg(feature = "server")]
impl GraphqlServerExt for Request {
    fn graphql_schema(&self) -> std::sync::Arc<SchemaHandle> {
        self.state::<SchemaHandle>()
    }

    fn try_graphql_schema(&self) -> Option<std::sync::Arc<SchemaHandle>> {
        self.try_state::<SchemaHandle>()
    }
}

pub struct GraphQlBound {
    client: std::sync::Arc<GraphQlClient>,
}

impl GraphQlBound {
    pub fn client(&self) -> &GraphQlClient {
        &self.client
    }

    pub fn query(&self, query: impl Into<String>) -> PendingGraphql {
        self.client.query(query)
    }

    pub fn mutation(&self, query: impl Into<String>) -> PendingGraphql {
        self.client.mutation(query)
    }

    pub async fn execute(
        &self,
        query: impl Into<String>,
    ) -> Result<serde_json::Value, GraphqlError> {
        self.query(query).data().await
    }
}
