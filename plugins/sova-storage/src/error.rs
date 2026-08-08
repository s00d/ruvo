use sova_core::extend::ErrorResponse;
use sova_core::{Error, IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("{0}")]
    Msg(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<StorageError> for Error {
    fn from(err: StorageError) -> Self {
        Error::Internal(err.to_string())
    }
}

impl IntoResponse for StorageError {
    fn into_response(self) -> Response {
        Error::from(self).into_response()
    }
}

impl ErrorResponse for StorageError {}

#[cfg(any(feature = "s3", feature = "gcs", feature = "azure"))]
impl From<opendal::Error> for StorageError {
    fn from(err: opendal::Error) -> Self {
        StorageError::Msg(err.to_string())
    }
}
