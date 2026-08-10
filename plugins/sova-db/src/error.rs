use sea_orm::{DbErr, SqlErr};
use sova_core::extend::ErrorResponse;
use sova_core::{Error, IntoResponse, Response};

/// Newtype around SeaORM [`DbErr`] mapped to HTTP via [`Error::Response`].
#[derive(Debug)]
pub struct DbError(pub DbErr);

impl From<DbErr> for DbError {
    fn from(err: DbErr) -> Self {
        Self(err)
    }
}

fn map_db_err(err: DbErr) -> Error {
    let (status, public) = if matches!(err, DbErr::RecordNotFound(_)) {
        (404, "Not Found")
    } else if let Some(sql) = err.sql_err() {
        match sql {
            SqlErr::UniqueConstraintViolation(_) => (409, "Conflict"),
            SqlErr::ForeignKeyConstraintViolation(_) => (422, "Unprocessable Entity"),
            _ => match &err {
                DbErr::ConnectionAcquire(_) | DbErr::Conn(_) => (503, "Service Unavailable"),
                _ => (500, "Internal Server Error"),
            },
        }
    } else {
        match &err {
            DbErr::ConnectionAcquire(_) | DbErr::Conn(_) => (503, "Service Unavailable"),
            _ => (500, "Internal Server Error"),
        }
    };
    if status >= 500 {
        tracing::error!(error = %err, "database error");
    }
    Error::Response(Box::new(Response::text(public).status(status)))
}

impl From<DbError> for Error {
    fn from(err: DbError) -> Self {
        map_db_err(err.0)
    }
}

impl IntoResponse for DbError {
    fn into_response(self) -> Response {
        Error::from(self).into_response()
    }
}

impl ErrorResponse for DbError {}
