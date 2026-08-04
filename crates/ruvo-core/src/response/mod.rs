mod file;
mod typed;

pub use typed::{Html, Json, NoContent, Redirect, Text};

use crate::error::IntoResponse;
use bytes::Bytes;
use futures_util::TryStreamExt;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use http_body::Frame;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use serde::Serialize;
use std::convert::Infallible;
use std::mem;
use std::path::Path;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
/// Boxed HTTP body used for both request streams and response streams.
pub type HttpBody = BoxBody<Bytes, BoxError>;
/// Alias for [`HttpBody`] (historical name for response streaming).
pub type ResponseBody = HttpBody;

/// Express-style HTTP response.
pub struct Response {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Body,
}

pub enum Body {
    Bytes(Bytes),
    Stream(ResponseBody),
}

impl From<Bytes> for Body {
    fn from(b: Bytes) -> Self {
        Body::Bytes(b)
    }
}

impl From<Vec<u8>> for Body {
    fn from(b: Vec<u8>) -> Self {
        Body::Bytes(Bytes::from(b))
    }
}

impl From<&'static [u8]> for Body {
    fn from(b: &'static [u8]) -> Self {
        Body::Bytes(Bytes::from_static(b))
    }
}

impl From<String> for Body {
    fn from(s: String) -> Self {
        Body::Bytes(Bytes::from(s))
    }
}

impl Body {
    /// Buffer the entire body into memory (pays for streams intentionally).
    pub async fn collect(self) -> Result<Bytes, BoxError> {
        match self {
            Body::Bytes(b) => Ok(b),
            Body::Stream(stream) => {
                let collected = BodyExt::collect(stream).await?;
                Ok(collected.to_bytes())
            }
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::empty()
    }
}

impl Response {
    pub fn empty() -> Self {
        Self {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Body::Bytes(Bytes::new()),
        }
    }

    pub fn text(body: impl Into<String>) -> Self {
        let mut res = Self::empty();
        res.set_text(body.into());
        res
    }

    pub fn html(body: impl Into<String>) -> Self {
        let mut res = Self::empty();
        res.headers.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        res.body = Body::Bytes(Bytes::from(body.into()));
        res
    }

    pub fn json<T: Serialize>(value: &T) -> Self {
        match serde_json::to_vec(value) {
            Ok(bytes) => {
                let mut res = Self::empty();
                res.headers.insert(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                res.body = Body::Bytes(Bytes::from(bytes));
                res
            }
            Err(err) => Self::text(format!("JSON encode error: {err}")).status(500),
        }
    }

    pub fn redirect(location: impl AsRef<str>) -> Self {
        Redirect::to(location.as_ref()).into_response()
    }

    /// Buffered body with an explicit MIME type.
    pub fn bytes(data: impl Into<Bytes>, mime: &str) -> Self {
        let mut res = Self::empty();
        if let Ok(v) = HeaderValue::from_str(mime) {
            res.headers.insert(http::header::CONTENT_TYPE, v);
        }
        let data = data.into();
        if let Ok(v) = HeaderValue::from_str(&data.len().to_string()) {
            res.headers.insert(http::header::CONTENT_LENGTH, v);
        }
        res.body = Body::Bytes(data);
        res
    }

    /// Set `Content-Disposition: attachment` for downloads.
    pub fn attachment(mut self, filename: &str) -> Self {
        let safe = filename.replace(['"', '\r', '\n', '\\'], "_");
        let value = format!("attachment; filename=\"{safe}\"");
        if let Ok(v) = HeaderValue::from_str(&value) {
            self.headers.insert(http::header::CONTENT_DISPOSITION, v);
        }
        self
    }

    /// Server-Sent Events stream (`text/event-stream`).
    ///
    /// Each item becomes one `data:` event (multi-line values split across `data:` lines).
    pub fn sse<S, E>(stream: S) -> Self
    where
        S: futures_util::Stream<Item = Result<String, E>> + Send + Sync + 'static,
        E: Into<BoxError> + Send + 'static,
    {
        use futures_util::StreamExt;
        let mapped = stream.map(|item| {
            item.map(|s| {
                let mut out = String::new();
                for line in s.split('\n') {
                    out.push_str("data: ");
                    out.push_str(line);
                    out.push('\n');
                }
                out.push('\n');
                Bytes::from(out)
            })
            .map_err(Into::into)
        });
        let mapped = mapped.map_ok(Frame::data).map_err(|e: BoxError| e);
        let mut res = Self::stream(BodyExt::boxed(StreamBody::new(mapped)));
        res.headers.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        res.headers.insert(
            http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        );
        res
    }

    pub fn stream(body: ResponseBody) -> Self {
        let mut res = Self::empty();
        res.body = Body::Stream(body);
        res
    }

    pub fn from_reader_stream<S>(stream: S) -> Self
    where
        S: futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Send + Sync + 'static,
    {
        let mapped = stream.map_ok(Frame::data).map_err(|e| -> BoxError { Box::new(e) });
        Self::stream(BodyExt::boxed(StreamBody::new(mapped)))
    }

    /// Chainable status: `Response::json(&x).status(201)`.
    pub fn status(mut self, code: u16) -> Self {
        self.status = StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        self
    }

    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_ref().as_bytes()),
            HeaderValue::from_str(value.as_ref()),
        ) {
            self.headers.insert(name, value);
        }
        self
    }

    pub fn status_code(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    pub fn take_body(&mut self) -> Body {
        mem::replace(&mut self.body, Body::Bytes(Bytes::new()))
    }

    pub fn set_body(&mut self, body: impl Into<Body>) {
        self.body = body.into();
    }

    /// Body bytes when the response is buffered; `None` for streams.
    pub fn body_bytes(&self) -> Option<&[u8]> {
        match &self.body {
            Body::Bytes(b) => Some(b.as_ref()),
            Body::Stream(_) => None,
        }
    }

    /// Stream a local file (path-traversal safe relative to its parent directory).
    pub async fn file(path: impl AsRef<Path>) -> Self {
        file::serve_path(path.as_ref()).await
    }

    /// Stream a file under `dir` / `relative` (path-traversal safe).
    pub async fn file_in(dir: impl AsRef<Path>, relative: impl AsRef<Path>) -> Self {
        file::serve_in(dir.as_ref(), relative.as_ref()).await
    }

    pub(crate) fn clear_body(&mut self) {
        self.body = Body::Bytes(Bytes::new());
    }

    fn set_text(&mut self, body: String) {
        self.headers.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        self.body = Body::Bytes(Bytes::from(body));
    }

    pub(crate) fn into_http_body(self) -> ResponseBody {
        match self.body {
            Body::Bytes(b) => Full::new(b)
                .map_err(|_: Infallible| unreachable!())
                .boxed(),
            Body::Stream(b) => b,
        }
    }
}

