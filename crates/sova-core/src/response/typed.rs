//! Typed response wrappers and redirect helpers.

use super::Response;
use crate::error::IntoResponse;
use crate::request::Request;
use serde::Serialize;

/// HTML body (`text/html; charset=utf-8`).
pub struct Html<T>(pub T);

/// JSON body (`application/json`).
pub struct Json<T>(pub T);

/// Plain text (`text/plain; charset=utf-8`).
pub struct Text<T>(pub T);

/// Empty 204 response.
pub struct NoContent;

/// HTTP redirect with explicit status.
pub struct Redirect {
    status: u16,
    location: String,
}

impl Redirect {
    pub fn to(location: impl Into<String>) -> Self {
        Self {
            status: 302,
            location: location.into(),
        }
    }

    pub fn permanent(location: impl Into<String>) -> Self {
        Self {
            status: 301,
            location: location.into(),
        }
    }

    pub fn see_other(location: impl Into<String>) -> Self {
        Self {
            status: 303,
            location: location.into(),
        }
    }

    pub fn with(status: u16, location: impl Into<String>) -> Self {
        Self {
            status,
            location: location.into(),
        }
    }

    /// 303 to `Referer`, or `/` when missing.
    pub fn back(req: &Request) -> Self {
        Self::back_or(req, "/")
    }

    /// 303 to `Referer`, or `fallback` when missing/empty.
    pub fn back_or(req: &Request, fallback: impl Into<String>) -> Self {
        Self::see_other(referer_or(req, fallback))
    }
}

/// `Referer` header value, or `fallback` when absent/empty.
pub fn referer_or(req: &Request, fallback: impl Into<String>) -> String {
    req.header("referer")
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| fallback.into())
}

impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        Response::empty()
            .status(self.status)
            .header("location", &self.location)
    }
}

impl<T: Into<String>> IntoResponse for Html<T> {
    fn into_response(self) -> Response {
        Response::html(self.0.into())
    }
}

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        Response::json(&self.0)
    }
}

impl<T: Into<String>> IntoResponse for Text<T> {
    fn into_response(self) -> Response {
        Response::text(self.0.into())
    }
}

impl IntoResponse for NoContent {
    fn into_response(self) -> Response {
        Response::empty().status(204)
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::empty().status(204)
    }
}

impl<T: IntoResponse> IntoResponse for Option<T> {
    fn into_response(self) -> Response {
        match self {
            Some(v) => v.into_response(),
            None => Response::text("Not Found").status(404),
        }
    }
}

impl<T: IntoResponse> IntoResponse for (u16, T) {
    fn into_response(self) -> Response {
        self.1.into_response().status(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::IntoResponse;
    use crate::Request;
    use http::Method;

    #[test]
    fn html_sets_content_type() {
        let res = Html("<b>x</b>".to_string()).into_response();
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(res.body_bytes(), Some(b"<b>x</b>".as_slice()));
    }

    #[test]
    fn json_encodes() {
        let res = Json(serde_json::json!({"ok": true})).into_response();
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "application/json"
        );
        assert!(res.body_bytes().unwrap().windows(4).any(|w| w == b"true"));
    }

    #[test]
    fn option_none_is_404() {
        let res = Option::<Html<String>>::None.into_response();
        assert_eq!(res.status_code().as_u16(), 404);
    }

    #[test]
    fn tuple_sets_status() {
        let res = (201, Json(serde_json::json!({"id": 1}))).into_response();
        assert_eq!(res.status_code().as_u16(), 201);
    }

    #[test]
    fn redirect_see_other() {
        let res = Redirect::see_other("/home").into_response();
        assert_eq!(res.status_code().as_u16(), 303);
        assert_eq!(res.headers().get("location").unwrap(), "/home");
    }

    #[test]
    fn redirect_back_uses_referer() {
        let req = Request::builder()
            .method(Method::POST)
            .path("/x")
            .header("referer", "/cabinet/notes")
            .build();
        let res = Redirect::back(&req).into_response();
        assert_eq!(res.status_code().as_u16(), 303);
        assert_eq!(res.headers().get("location").unwrap(), "/cabinet/notes");
    }

    #[test]
    fn redirect_back_fallback() {
        let req = Request::builder().method(Method::GET).path("/x").build();
        let res = Redirect::back_or(&req, "/home").into_response();
        assert_eq!(res.headers().get("location").unwrap(), "/home");
    }
}
