//! GraphQL client errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphqlError {
    #[error("graphql transport: {0}")]
    Transport(String),
    #[error("graphql http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("graphql errors: {0}")]
    Graphql(String),
    #[error("graphql decode: {0}")]
    Decode(String),
    #[error("graphql plugin not installed")]
    NotInstalled,
}

impl From<GraphqlError> for sova_core::Error {
    fn from(value: GraphqlError) -> Self {
        sova_core::Error::Internal(value.to_string())
    }
}
