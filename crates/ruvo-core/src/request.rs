use crate::error::{Error, Result};
use crate::response::HttpBody;
use crate::server::collect_limited;
use crate::state::{Extensions, StateMap};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

/// Request body: buffered bytes or a lazy stream (collected on demand).
pub enum ReqBody {
    Bytes(Bytes),
    Stream(HttpBody),
    /// Consumed by a prior body reader (`by` names the consumer).
    Taken { by: &'static str },
}

/// Incoming HTTP request with Express-style helpers.
pub struct Request {
    pub method: Method,
    pub path: String,
    pub headers: HeaderMap,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    /// Scheme (`http` / `https`), possibly from `X-Forwarded-Proto` when trust_proxy.
    pub(crate) scheme: String,
    /// Host (no port stripping beyond what the client sent).
    pub(crate) host: String,
    /// Raw query string without `?` (for `query_as`).
    pub(crate) raw_query: String,
    pub(crate) body: ReqBody,
    pub(crate) body_limit: usize,
    pub(crate) state: Arc<StateMap>,
    pub(crate) extensions: Extensions,
}

/// Builder for test / embedded requests. `state` and `extensions` stay empty —
/// [`crate::App::handle`] fills router state.
pub struct RequestBuilder {
    method: Method,
    path: String,
    headers: HeaderMap,
    body: Bytes,
    query: HashMap<String, String>,
    raw_query: String,
    scheme: String,
    host: String,
    body_limit: usize,
}

impl RequestBuilder {
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        let path = path.into();
        match path.split_once('?') {
            Some((p, q)) => {
                self.path = p.to_string();
                self.raw_query = q.to_string();
                self.query = parse_query(q);
            }
            None => {
                self.path = path;
            }
        }
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

    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    pub fn body_limit(mut self, limit: usize) -> Self {
        self.body_limit = limit;
        self
    }

    pub fn query_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(key.into(), value.into());
        self.raw_query = serde_urlencoded::to_string(&self.query).unwrap_or_default();
        self
    }

    pub fn scheme(mut self, scheme: impl Into<String>) -> Self {
        self.scheme = scheme.into();
        self
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn build(self) -> Request {
        Request {
            method: self.method,
            path: self.path,
            headers: self.headers,
            params: HashMap::new(),
            query: self.query,
            scheme: self.scheme,
            host: self.host,
            raw_query: self.raw_query,
            body: ReqBody::Bytes(self.body),
            body_limit: self.body_limit,
            state: Arc::new(StateMap::new()),
            extensions: Extensions::new(),
        }
    }
}

impl Request {
    /// Build an empty request (tests / embedded). `App::handle` injects router state.
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Request::builder().method(method).path(path).build()
    }

    pub fn builder() -> RequestBuilder {
        RequestBuilder {
            method: Method::GET,
            path: "/".into(),
            headers: HeaderMap::new(),
            body: Bytes::new(),
            query: HashMap::new(),
            raw_query: String::new(),
            scheme: "http".into(),
            host: "localhost".into(),
            body_limit: 2 * 1024 * 1024,
        }
    }

    /// Configured max body size (from the server / builder).
    pub fn body_limit(&self) -> usize {
        self.body_limit
    }

    /// Collect the full body as bytes (respecting [`Self::body_limit`]).
    pub async fn body(&mut self) -> Result<Bytes> {
        self.collect_body("body").await
    }

    /// Buffer the body if needed, then return UTF-8 text.
    pub async fn text(&mut self) -> Result<String> {
        let bytes = self.collect_body("text").await?;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| Error::BadRequest(format!("invalid UTF-8 body: {e}")))
    }

    pub async fn json<T: DeserializeOwned>(&mut self) -> Result<T> {
        let bytes = self.collect_body("json").await?;
        serde_json::from_slice(&bytes).map_err(Error::from)
    }

    /// Parse `application/x-www-form-urlencoded` body.
    pub async fn form<T: DeserializeOwned>(&mut self) -> Result<T> {
        let bytes = self.collect_body("form").await?;
        serde_urlencoded::from_bytes(&bytes)
            .map_err(|e| Error::BadRequest(format!("form error: {e}")))
    }

    /// Deserialize the query string into `T`.
    pub fn query_as<T: DeserializeOwned>(&self) -> Result<T> {
        serde_urlencoded::from_str(&self.raw_query)
            .map_err(|e| Error::BadRequest(format!("query error: {e}")))
    }

    pub fn query(&self, key: &str) -> Option<&str> {
        self.query.get(key).map(|s| s.as_str())
    }

    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    /// Parse a path param (`FromStr`), or `BadRequest`.
    pub fn param_as<T: FromStr>(&self, key: &str) -> Result<T>
    where
        T::Err: std::fmt::Display,
    {
        let raw = self
            .param(key)
            .ok_or_else(|| Error::BadRequest(format!("missing param `{key}`")))?;
        raw.parse()
            .map_err(|e| Error::BadRequest(format!("param `{key}`: {e}")))
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
            .map(|v| v.split(';').next().unwrap_or(v).trim())
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn is_secure(&self) -> bool {
        self.scheme.eq_ignore_ascii_case("https")
    }

    /// Absolute URL for this request path (no query).
    pub fn url(&self) -> String {
        if self.raw_query.is_empty() {
            format!("{}://{}{}", self.scheme, self.host, self.path)
        } else {
            format!(
                "{}://{}{}?{}",
                self.scheme, self.host, self.path, self.raw_query
            )
        }
    }

    /// Take the body as a stream (once). Subsequent body reads fail.
    pub fn into_body_stream(&mut self) -> Result<HttpBody> {
        self.into_body_stream_as("into_body_stream")
    }

    /// Like [`Self::into_body_stream`], recording `by` in later "already consumed" errors.
    pub fn into_body_stream_as(&mut self, by: &'static str) -> Result<HttpBody> {
        match std::mem::replace(&mut self.body, ReqBody::Taken { by }) {
            ReqBody::Stream(s) => Ok(s),
            ReqBody::Bytes(b) => Ok(http_body_util::Full::new(b)
                .map_err(|_: std::convert::Infallible| unreachable!())
                .boxed()),
            ReqBody::Taken { by: prev } => Err(Error::BadRequest(format!(
                "body already consumed by {prev}"
            ))),
        }
    }

    async fn collect_body(&mut self, by: &'static str) -> Result<Bytes> {
        match std::mem::replace(&mut self.body, ReqBody::Taken { by }) {
            ReqBody::Bytes(b) => {
                self.body = ReqBody::Bytes(b.clone());
                Ok(b)
            }
            ReqBody::Stream(stream) => {
                let collected = collect_limited(stream, self.body_limit).await?;
                self.body = ReqBody::Bytes(collected.clone());
                Ok(collected)
            }
            ReqBody::Taken { by: prev } => Err(Error::BadRequest(format!(
                "body already consumed by {prev}"
            ))),
        }
    }

    /// Shared app state. Panics if the type was never registered via `app.state`.
    pub fn state<T>(&self) -> Arc<T>
    where
        T: Send + Sync + 'static,
    {
        self.try_state().unwrap_or_else(|| {
            panic!(
                "state `{}` is not registered — call app.state(..)",
                std::any::type_name::<T>()
            )
        })
    }

    pub fn try_state<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.state.get::<T>()
    }

    /// Store a per-request value (e.g. from auth middleware).
    pub fn set<T: Send + Sync + 'static>(&mut self, value: T) {
        self.extensions.insert(value);
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions.get::<T>()
    }

    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.extensions.get_mut::<T>()
    }

    pub fn take<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.extensions.remove::<T>()
    }

    /// Typed metadata from the matched route (`route_meta` / plugin helpers).
    pub fn route_meta<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.get::<crate::state::MatchedMeta>()
            .and_then(|m| m.0.get::<T>())
    }
}

/// Parse query string with `+` → space (via serde_urlencoded).
pub fn parse_query(query: &str) -> HashMap<String, String> {
    serde_urlencoded::from_str::<HashMap<String, String>>(query).unwrap_or_default()
}

pub fn percent_decode(input: &str) -> String {
    percent_encoding::percent_decode_str(input)
        .decode_utf8_lossy()
        .into_owned()
}

/// Build scheme/host from the incoming request (and proxy headers when trusted).
pub(crate) fn resolve_scheme_host(
    headers: &HeaderMap,
    uri_scheme: Option<&str>,
    trust_proxy: bool,
) -> (String, String) {
    let mut scheme = uri_scheme.unwrap_or("http").to_string();
    let mut host = headers
        .get(http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost")
        .to_string();

    if trust_proxy {
        if let Some(proto) = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            scheme = proto.to_string();
        }
        if let Some(h) = headers
            .get("x-forwarded-host")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            host = h.to_string();
        }
    }
    (scheme, host)
}
