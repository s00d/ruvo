//! GraphQL for Sova — **client first**, optional schema mount.
//!
//! ```ignore
//! use sova_graphql::{FakeGraphql, GraphQl, GraphQlExt};
//! use serde_json::json;
//!
//! let fake = FakeGraphql::new().stub("hello", json!({ "hello": "world" }));
//! app.install(GraphQl::fake(fake.clone()));
//!
//! // in a handler:
//! let v = req.graphql().query("query { hello }").await?;
//! assert_eq!(v["hello"], "world");
//! ```

mod bound;
mod client;
mod error;
mod fake;
mod plugin;
#[cfg(feature = "server")]
mod server;

pub use bound::{GraphQlBound, GraphQlExt};
pub use client::{GraphQlClient, GraphqlResponse};
pub use error::GraphqlError;
pub use fake::{FakeGraphql, GraphqlCall};
pub use plugin::GraphQl;

#[cfg(feature = "server")]
pub use server::{execute_request, GraphqlJsonRequest, SchemaHandle};

#[cfg(feature = "server")]
pub use async_graphql;
