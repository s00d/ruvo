//! Rabbit errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RabbitError {
    #[error("rabbit: {0}")]
    Msg(String),
    #[error("rabbit not connected")]
    NotConnected,
    #[error("rabbit plugin not installed")]
    NotInstalled,
}

impl From<RabbitError> for sova_core::Error {
    fn from(value: RabbitError) -> Self {
        sova_core::Error::Internal(value.to_string())
    }
}
