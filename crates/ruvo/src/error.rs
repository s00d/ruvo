//! Facade application error — bridges domain crates into one `?` type.
//!
//! Core stays free of ORM/validation deps; `From` impls live here (orphan-rule safe).

use ruvo_core::extend::ErrorResponse;
use ruvo_core::{Error, IntoResponse, Response};

/// Unified handler / `main` error for apps that use the `ruvo` facade.
#[derive(Debug)]
pub enum AppError {
    Core(Error),
}

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        Self::Core(err)
    }
}

#[cfg(feature = "db")]
impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        Self::Core(ruvo_db::DbError::from(err).into())
    }
}

#[cfg(feature = "db")]
impl From<ruvo_db::DbError> for AppError {
    fn from(err: ruvo_db::DbError) -> Self {
        Self::Core(err.into())
    }
}

#[cfg(feature = "vld")]
impl From<ruvo_vld::ValidationError> for AppError {
    fn from(err: ruvo_vld::ValidationError) -> Self {
        Self::Core(err.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Core(err) => err.into_response(),
        }
    }
}

impl ErrorResponse for AppError {}

/// Facade result type (`Result<T, AppError>`).
pub type Result<T> = std::result::Result<T, AppError>;
