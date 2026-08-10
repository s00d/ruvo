//! RFC 7807 Problem Details helpers (`application/problem+json`).

use crate::request_id::current_request_id;
use crate::response::Response;
use serde::Serialize;
use serde_json::{json, Map, Value};

/// Build a Problem Details response.
///
/// `extensions` are merged into the JSON object (e.g. `errors`, `instance`).
pub fn problem_response(
    status: u16,
    title: impl Into<String>,
    detail: impl Into<String>,
    extensions: impl IntoIterator<Item = (String, Value)>,
) -> Response {
    let title = title.into();
    let detail = detail.into();
    let mut body = Map::new();
    body.insert("type".into(), json!("about:blank"));
    body.insert("title".into(), json!(title));
    body.insert("status".into(), json!(status));
    body.insert("detail".into(), json!(detail));
    if let Some(rid) = current_request_id() {
        body.insert("request_id".into(), json!(rid));
    }
    for (k, v) in extensions {
        body.insert(k, v);
    }
    Response::json(&Value::Object(body))
        .status(status)
        .header("content-type", "application/problem+json")
}

/// Map a framework [`crate::Error`] to Problem Details (API preset).
pub fn error_to_problem(err: crate::Error) -> Response {
    use crate::Error;
    match err {
        Error::Response(res) => *res,
        Error::NotFound => problem_response(404, "Not Found", "Not Found", []),
        Error::Unauthorized => problem_response(401, "Unauthorized", "Unauthorized", []),
        Error::Forbidden => problem_response(403, "Forbidden", "Forbidden", []),
        Error::BadRequest(msg) => problem_response(400, "Bad Request", msg, []),
        Error::PayloadTooLarge => {
            problem_response(413, "Payload Too Large", "Payload Too Large", [])
        }
        Error::MethodNotAllowed => {
            problem_response(405, "Method Not Allowed", "Method Not Allowed", [])
        }
        Error::Internal(msg) => problem_response(500, "Internal Server Error", msg, []),
        Error::Json(e) => problem_response(400, "Bad Request", format!("JSON error: {e}"), []),
        Error::Io(e) => {
            problem_response(500, "Internal Server Error", format!("IO error: {e}"), [])
        }
    }
}

/// Convenience for a field-error list (`errors` array).
pub fn problem_with_errors<E: Serialize>(
    status: u16,
    title: impl Into<String>,
    detail: impl Into<String>,
    errors: &[E],
) -> Response {
    let errors = serde_json::to_value(errors).unwrap_or(json!([]));
    problem_response(status, title, detail, [("errors".into(), errors)])
}
