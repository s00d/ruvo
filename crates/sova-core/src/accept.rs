//! Accept-aware error responses (HTML / problem+json / text).

use crate::error::Error;
use crate::problem::error_to_problem;
use crate::request_id::current_request_id;
use crate::response::Response;
use std::cell::RefCell;

tokio::task_local! {
    static ACCEPT: RefCell<String>;
    static PATH: RefCell<String>;
}

/// Run `fut` with the request `Accept` header visible to [`current_accept`].
pub async fn with_accept<F, T>(accept: impl Into<String>, fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    ACCEPT.scope(RefCell::new(accept.into()), fut).await
}

/// Like [`with_accept`], also recording the request path for API-aware error negotiation.
pub async fn with_request_meta<F, T>(
    accept: impl Into<String>,
    path: impl Into<String>,
    fut: F,
) -> T
where
    F: std::future::Future<Output = T>,
{
    ACCEPT
        .scope(RefCell::new(accept.into()), async move {
            PATH.scope(RefCell::new(path.into()), fut).await
        })
        .await
}

/// Accept header captured for the current request (if any).
pub fn current_accept() -> Option<String> {
    ACCEPT.try_with(|c| c.borrow().clone()).ok()
}

/// Request path captured for the current request (if any).
pub fn current_path() -> Option<String> {
    PATH.try_with(|c| c.borrow().clone()).ok()
}

/// Preferred error body format from an `Accept` header value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorFormat {
    Html,
    ProblemJson,
    Text,
}

/// Pick a format: `text/html` wins over json/problem when both appear without clear exclusivity;
/// `application/problem+json` / `application/json` → problem; else text.
pub fn negotiate_error_format(accept: Option<&str>) -> ErrorFormat {
    let accept = accept.unwrap_or("*/*").to_ascii_lowercase();
    let wants_html = accept.contains("text/html");
    let wants_problem = accept.contains("application/problem+json");
    let wants_json = accept.contains("application/json");

    if wants_html && !accept.contains("application/json") && !wants_problem {
        return ErrorFormat::Html;
    }
    // Prefer HTML when client lists html early (browsers).
    if wants_html {
        if let Some(html_i) = accept.find("text/html") {
            let json_i = accept
                .find("application/json")
                .or_else(|| accept.find("application/problem+json"))
                .unwrap_or(usize::MAX);
            if html_i <= json_i {
                return ErrorFormat::Html;
            }
        }
    }
    if wants_problem || wants_json {
        return ErrorFormat::ProblemJson;
    }
    if accept.trim() == "*/*" || accept.is_empty() {
        return ErrorFormat::Text;
    }
    ErrorFormat::Text
}

fn error_title_detail(err: &Error) -> (u16, &'static str, String) {
    match err {
        Error::NotFound => (404, "Not Found", "Not Found".into()),
        Error::Unauthorized => (401, "Unauthorized", "Unauthorized".into()),
        Error::Forbidden => (403, "Forbidden", "Forbidden".into()),
        Error::BadRequest(msg) => (400, "Bad Request", msg.clone()),
        Error::PayloadTooLarge => (413, "Payload Too Large", "Payload Too Large".into()),
        Error::MethodNotAllowed => (405, "Method Not Allowed", "Method Not Allowed".into()),
        Error::Internal(msg) => (500, "Internal Server Error", msg.clone()),
        Error::Json(e) => (400, "Bad Request", format!("JSON error: {e}")),
        Error::Io(e) => (500, "Internal Server Error", format!("IO error: {e}")),
        Error::Response(_) => (500, "Error", "Error".into()),
    }
}

/// Minimal HTML error document (no templates).
pub fn html_error_page(status: u16, title: &str, detail: &str) -> Response {
    let rid = current_request_id().unwrap_or_default();
    let rid_row = if rid.is_empty() {
        String::new()
    } else {
        format!("<p class=\"rid\">request_id: {rid}</p>")
    };
    let body = format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <title>{status} {title}</title>\
         <style>body{{font-family:system-ui,sans-serif;margin:2rem;color:#111}}\
         h1{{font-size:1.5rem}} .rid{{color:#666;font-size:.875rem}}</style></head>\
         <body><h1>{status} {title}</h1><p>{detail}</p>{rid_row}</body></html>",
        detail = html_escape(detail),
        title = html_escape(title),
    );
    Response::html(body).status(status)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Map [`Error`] using `Accept` (or [`current_accept`] when `accept` is `None`).
///
/// Paths under `/api/` always get problem+json so browser address-bar hits on JSON
/// APIs (common in SPA proxies) do not negotiate HTML error pages.
pub fn error_response_for_accept(accept: Option<&str>, err: Error) -> Response {
    if let Error::Response(res) = err {
        return *res;
    }
    let path = current_path();
    if path
        .as_deref()
        .is_some_and(|p| p == "/api" || p.starts_with("/api/"))
    {
        return error_to_problem(err);
    }
    let accept = accept.map(|s| s.to_string()).or_else(current_accept);
    match negotiate_error_format(accept.as_deref()) {
        ErrorFormat::Html => {
            let (status, title, detail) = error_title_detail(&err);
            html_error_page(status, title, &detail)
        }
        ErrorFormat::ProblemJson => error_to_problem(err),
        ErrorFormat::Text => err.into_response(),
    }
}

/// Status-line response (router 404/405) negotiated from request Accept.
pub fn status_response_for_accept(accept: Option<&str>, status: u16, detail: &str) -> Response {
    let title = match status {
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    match negotiate_error_format(accept) {
        ErrorFormat::Html => html_error_page(status, title, detail),
        ErrorFormat::ProblemJson => crate::problem::problem_response(status, title, detail, []),
        ErrorFormat::Text => Response::text(detail.to_string()).status(status),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_accept_prefers_html() {
        assert_eq!(
            negotiate_error_format(Some(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
            )),
            ErrorFormat::Html
        );
    }

    #[test]
    fn api_client_prefers_json() {
        assert_eq!(
            negotiate_error_format(Some("application/json")),
            ErrorFormat::ProblemJson
        );
        assert_eq!(
            negotiate_error_format(Some("application/problem+json")),
            ErrorFormat::ProblemJson
        );
    }
}
