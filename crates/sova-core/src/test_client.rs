//! In-process HTTP client with a cookie jar (feature `testing`).

use crate::app::{App, Server};
use crate::error::Result;
use crate::request::Request;
use crate::response::Response;
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use std::collections::HashMap;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Mutates a request before [`Server::handle`] (e.g. inject auth extensions).
pub type RequestHook = Arc<dyn Fn(&mut Request) + Send + Sync>;

/// Test client over a compiled [`Server`], always tracking cookies.
pub struct TestClient {
    server: Server,
    jar: Mutex<HashMap<String, String>>,
    request_hooks: Mutex<Vec<RequestHook>>,
}

impl TestClient {
    pub fn new(app: App) -> Result<Self> {
        Ok(Self {
            server: app.build()?,
            jar: Mutex::new(HashMap::new()),
            request_hooks: Mutex::new(Vec::new()),
        })
    }

    /// Same as [`Self::new`] (Rocket-style name).
    pub fn tracked(app: App) -> Result<Self> {
        Self::new(app)
    }

    pub fn server(&self) -> &Server {
        &self.server
    }

    /// Run `hook` on every request before dispatch (stacked; call order preserved).
    pub fn on_request<F>(&self, hook: F)
    where
        F: Fn(&mut Request) + Send + Sync + 'static,
    {
        self.request_hooks.lock().unwrap().push(Arc::new(hook));
    }

    /// Drop all [`Self::on_request`] hooks.
    pub fn clear_request_hooks(&self) {
        self.request_hooks.lock().unwrap().clear();
    }

    pub fn get(&self, path: impl Into<String>) -> ClientRequest<'_> {
        ClientRequest::new(self, Method::GET, path.into())
    }

    pub fn post(&self, path: impl Into<String>) -> ClientRequest<'_> {
        ClientRequest::new(self, Method::POST, path.into())
    }

    pub fn put(&self, path: impl Into<String>) -> ClientRequest<'_> {
        ClientRequest::new(self, Method::PUT, path.into())
    }

    pub fn patch(&self, path: impl Into<String>) -> ClientRequest<'_> {
        ClientRequest::new(self, Method::PATCH, path.into())
    }

    pub fn delete(&self, path: impl Into<String>) -> ClientRequest<'_> {
        ClientRequest::new(self, Method::DELETE, path.into())
    }

    fn cookie_header(&self) -> Option<String> {
        let jar = self.jar.lock().unwrap();
        if jar.is_empty() {
            return None;
        }
        Some(
            jar.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    fn store_set_cookie(&self, res: &Response) {
        let mut jar = self.jar.lock().unwrap();
        for val in res.headers().get_all(http::header::SET_COOKIE) {
            let Ok(raw) = val.to_str() else { continue };
            let pair = raw.split(';').next().unwrap_or(raw).trim();
            if let Some((name, value)) = pair.split_once('=') {
                jar.insert(name.trim().to_string(), value.trim().to_string());
            }
        }
    }

    fn apply_hooks(&self, req: &mut Request) {
        let hooks = self.request_hooks.lock().unwrap().clone();
        for hook in hooks {
            hook(req);
        }
    }
}

/// Builder for a single request; `.await` sends it via [`IntoFuture`].
pub struct ClientRequest<'a> {
    client: &'a TestClient,
    method: Method,
    path: String,
    headers: HeaderMap,
    body: Bytes,
}

impl<'a> ClientRequest<'a> {
    fn new(client: &'a TestClient, method: Method, path: String) -> Self {
        Self {
            client,
            method,
            path,
            headers: HeaderMap::new(),
            body: Bytes::new(),
        }
    }

    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_ref().as_bytes()),
            HeaderValue::from_str(value.as_ref()),
        ) {
            self.headers.insert(n, v);
        }
        self
    }

    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    pub fn form(mut self, pairs: &[(&str, &str)]) -> Self {
        let encoded = serde_urlencoded::to_string(pairs).unwrap_or_default();
        self.headers.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        self.body = Bytes::from(encoded);
        self
    }

    pub fn json<T: serde::Serialize>(mut self, value: &T) -> Self {
        let bytes = serde_json::to_vec(value).unwrap_or_default();
        self.headers.insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        self.body = Bytes::from(bytes);
        self
    }

    async fn dispatch(self) -> Response {
        let mut builder = Request::builder()
            .method(self.method)
            .path(self.path)
            .body(self.body);
        for (k, v) in self.headers.iter() {
            if let Ok(s) = v.to_str() {
                builder = builder.header(k.as_str(), s);
            }
        }
        if let Some(cookie) = self.client.cookie_header() {
            builder = builder.header("cookie", cookie);
        }
        let mut req = builder.build();
        self.client.apply_hooks(&mut req);
        let res = self.client.server.handle(req).await;
        self.client.store_set_cookie(&res);
        res
    }
}

impl<'a> IntoFuture for ClientRequest<'a> {
    type Output = Response;
    type IntoFuture = Pin<Box<dyn Future<Output = Response> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.dispatch())
    }
}

/// Fluent assertions for HTTP responses in tests.
pub trait ResponseAssert {
    /// Panic unless status matches `code`.
    fn assert_status(&self, code: u16) -> &Self;

    /// Deserialize buffered JSON body; panics on failure.
    fn json<T: serde::de::DeserializeOwned>(&self) -> T;

    /// Parse buffered body as [`serde_json::Value`].
    fn json_value(&self) -> serde_json::Value;
}

impl ResponseAssert for Response {
    fn assert_status(&self, code: u16) -> &Self {
        let got = self.status_code().as_u16();
        assert_eq!(
            got,
            code,
            "unexpected status {got}, body: {:?}",
            self.body_bytes().map(|b| String::from_utf8_lossy(b).into_owned())
        );
        self
    }

    fn json<T: serde::de::DeserializeOwned>(&self) -> T {
        let bytes = self
            .body_bytes()
            .unwrap_or_else(|| panic!("response body is not buffered"));
        serde_json::from_slice(bytes).unwrap_or_else(|e| {
            panic!(
                "json decode failed: {e}; body: {}",
                String::from_utf8_lossy(bytes)
            )
        })
    }

    fn json_value(&self) -> serde_json::Value {
        self.json()
    }
}
