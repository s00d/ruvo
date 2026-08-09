use sova_core::extend::ErrorResponse;
use sova_core::{Error, IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("not found")]
    NotFound,
    #[error("forbidden path")]
    Forbidden,
    #[error("{0}")]
    Msg(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<FsError> for Error {
    fn from(err: FsError) -> Self {
        match err {
            FsError::NotFound => Error::NotFound,
            FsError::Forbidden => Error::Forbidden,
            FsError::Msg(m) => Error::Internal(m),
            FsError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => Error::NotFound,
            FsError::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Error::Forbidden,
            FsError::Io(e) => Error::Internal(e.to_string()),
        }
    }
}

impl IntoResponse for FsError {
    fn into_response(self) -> Response {
        Error::from(self).into_response()
    }
}

impl ErrorResponse for FsError {}
