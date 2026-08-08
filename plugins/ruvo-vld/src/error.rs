use ruvo_core::extend::ErrorResponse;
use ruvo_core::{Error, IntoResponse, Json, Request, Response};
use serde::Serialize;
use vld::error::{IssueCode, PathSegment, ValidationIssue, VldError};

/// Newtype so Ruvo can implement [`IntoResponse`] (orphan rule).
#[derive(Debug, Clone)]
pub struct ValidationError(pub VldError);

impl From<VldError> for ValidationError {
    fn from(err: VldError) -> Self {
        Self(err)
    }
}

impl From<Error> for ValidationError {
    fn from(err: Error) -> Self {
        match err {
            Error::PayloadTooLarge => Self(VldError::single(
                IssueCode::TooBig {
                    maximum: 0.0,
                    inclusive: true,
                },
                "Payload Too Large",
            )),
            Error::BadRequest(msg) => Self(VldError::single(IssueCode::ParseError, msg)),
            other => Self(VldError::single(
                IssueCode::Custom {
                    code: "request_error".into(),
                },
                other.to_string(),
            )),
        }
    }
}

impl ValidationError {
    pub fn status_code(&self) -> u16 {
        if self.0.issues.iter().any(|i| {
            matches!(i.code, IssueCode::TooBig { .. }) && i.message.contains("Payload Too Large")
        }) {
            return 413;
        }
        if self.0.issues.iter().any(is_client_syntax) {
            400
        } else {
            422
        }
    }

    /// Build a response, optionally using Accept / flash (feature `flash`).
    pub fn respond(self, req: &Request) -> Response {
        #[cfg(feature = "flash")]
        {
            if wants_html(req) {
                return self.respond_flash(req);
            }
        }
        let _ = req;
        self.into_response()
    }

    #[cfg(feature = "flash")]
    fn respond_flash(self, req: &Request) -> Response {
        use ruvo_core::{FormData, Redirect};
        use ruvo_session::SessionExt;
        use serde_json::json;

        let session = req.session();
        let mut errors = serde_json::Map::new();
        for issue in &self.0.issues {
            let path = format_path(&issue.path);
            let key = if path.is_empty() {
                "_form".into()
            } else {
                path
            };
            errors.insert(key, json!(issue.message));
        }
        session.flash_errors(&serde_json::Value::Object(errors));

        let mut old = serde_json::Map::new();
        if let Some(data) = req.get::<FormData>() {
            for (k, values) in data.text_map() {
                match values.as_slice() {
                    [] => {}
                    [one] => {
                        old.insert(k.clone(), json!(one));
                    }
                    many => {
                        old.insert(
                            k.clone(),
                            json!(many.to_vec()),
                        );
                    }
                }
            }
        } else {
            for (k, v) in &req.query {
                old.insert(k.clone(), json!(v));
            }
            for (k, v) in &req.params {
                old.insert(k.clone(), json!(v));
            }
        }
        session.flash_old(&serde_json::Value::Object(old));

        Redirect::back(req).into_response()
    }
}

#[cfg(feature = "flash")]
fn wants_html(req: &Request) -> bool {
    let accept = req.header("accept").unwrap_or("*/*");
    if accept.contains("application/json") && !accept.contains("text/html") {
        return false;
    }
    accept.contains("text/html")
}

fn is_client_syntax(issue: &ValidationIssue) -> bool {
    match &issue.code {
        IssueCode::ParseError => true,
        IssueCode::InvalidType { .. } if issue.path.is_empty() => true,
        _ => false,
    }
}

pub(crate) fn format_path(path: &[PathSegment]) -> String {
    let mut out = String::new();
    for (i, seg) in path.iter().enumerate() {
        if i > 0 {
            out.push('.');
        }
        match seg {
            PathSegment::Field(name) => out.push_str(name),
            PathSegment::Index(idx) => out.push_str(&idx.to_string()),
        }
    }
    out
}

pub(crate) fn issue_code_slug(code: &IssueCode) -> String {
    match code {
        IssueCode::InvalidType { .. } => "invalid_type".into(),
        IssueCode::TooSmall { .. } => "too_small".into(),
        IssueCode::TooBig { .. } => "too_big".into(),
        IssueCode::InvalidString { validation } => format!("invalid_string_{validation:?}")
            .to_ascii_lowercase()
            .replace([' ', '{', '}'], ""),
        IssueCode::NotInt => "not_int".into(),
        IssueCode::NotFinite => "not_finite".into(),
        IssueCode::MissingField => "missing_field".into(),
        IssueCode::UnrecognizedField => "unrecognized_field".into(),
        IssueCode::IoError => "io_error".into(),
        IssueCode::ParseError => "parse_error".into(),
        IssueCode::Custom { code } => code.clone(),
    }
}

#[derive(Serialize)]
struct IssueBody {
    path: String,
    code: String,
    message: String,
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
    issues: Vec<IssueBody>,
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let issues = self
            .0
            .issues
            .into_iter()
            .map(|i| IssueBody {
                path: format_path(&i.path),
                code: issue_code_slug(&i.code),
                message: i.message,
            })
            .collect();
        let body = ErrorBody {
            error: "validation_failed",
            issues,
        };
        Json(body).into_response().status(status)
    }
}

impl From<ValidationError> for Error {
    fn from(err: ValidationError) -> Self {
        Error::Response(Box::new(err.into_response()))
    }
}

impl ErrorResponse for ValidationError {}
