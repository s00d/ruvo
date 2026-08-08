use ruvo_core::Error;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct RedisError(pub String);

impl RedisError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl From<redis::RedisError> for RedisError {
    fn from(err: redis::RedisError) -> Self {
        Self(err.to_string())
    }
}

impl From<RedisError> for Error {
    fn from(err: RedisError) -> Self {
        Error::Internal(err.0)
    }
}
