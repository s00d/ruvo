//! Facade application error — bridges domain crates into one `?` type.
//!
//! Core stays free of ORM/validation deps; `From` impls live here (orphan-rule safe).

use sova_core::extend::ErrorResponse;
use sova_core::{Error, IntoResponse, Response};

/// Unified handler / `main` error for apps that use the `sova` facade.
#[derive(Debug)]
pub enum AppError {
    Core(Error),
}

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        Self::Core(err)
    }
}

impl From<AppError> for Error {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Core(e) => e,
        }
    }
}

#[cfg(feature = "db")]
impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        Self::Core(sova_db::DbError::from(err).into())
    }
}

#[cfg(feature = "db")]
impl From<sova_db::DbError> for AppError {
    fn from(err: sova_db::DbError) -> Self {
        Self::Core(err.into())
    }
}

#[cfg(feature = "vld")]
impl From<sova_vld::ValidationError> for AppError {
    fn from(err: sova_vld::ValidationError) -> Self {
        Self::Core(err.into())
    }
}

#[cfg(feature = "storage")]
impl From<sova_storage::StorageError> for AppError {
    fn from(err: sova_storage::StorageError) -> Self {
        Self::Core(err.into())
    }
}

#[cfg(feature = "fs")]
impl From<sova_fs::FsError> for AppError {
    fn from(err: sova_fs::FsError) -> Self {
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
