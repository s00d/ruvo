//! `req.graphql()` — outbound GraphQL client bound to the request.

use crate::client::{GraphQlClient, PendingGraphql};
use crate::error::GraphqlError;
use sova_core::Request;

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
