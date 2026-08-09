use crate::response::Response;
use thiserror::Error as ThisError;

/// Framework and application errors mapped to HTTP responses.
#[derive(Debug, ThisError)]
pub enum Error {
    #[error("not found")]
    NotFound,

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("payload too large")]
    PayloadTooLarge,

    #[error("method not allowed")]
    MethodNotAllowed,

    #[error("internal error: {0}")]
    Internal(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Pre-built HTTP response (plugin/domain errors). Skips `error_handler`.
    #[error("response")]
    Response(Box<Response>),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Convenience for [`Error::BadRequest`].
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    /// Build [`Error::Response`] from a status and any [`IntoResponse`] body.
    pub fn custom(status: u16, body: impl IntoResponse) -> Self {
        Error::Response(Box::new(body.into_response().status(status)))
    }

    pub fn into_response(self) -> Response {
        match self {
            Error::NotFound => Response::text("Not Found").status(404),
            Error::Unauthorized => Response::text("Unauthorized").status(401),
            Error::Forbidden => Response::text("Forbidden").status(403),
            Error::BadRequest(msg) => Response::text(msg).status(400),
            Error::PayloadTooLarge => Response::text("Payload Too Large").status(413),
            Error::MethodNotAllowed => Response::text("Method Not Allowed").status(405),
            Error::Internal(msg) => Response::text(msg).status(500),
            Error::Json(err) => Response::text(format!("JSON error: {err}")).status(400),
            Error::Io(err) => Response::text(format!("IO error: {err}")).status(500),
            Error::Response(res) => *res,
        }
    }
}

/// Convert a value into an HTTP response.
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        Error::into_response(self)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::text(self)
    }
}

impl IntoResponse for &str {
    fn into_response(self) -> Response {
        Response::text(self)
    }
}
