//! CSRF protection via session double-submit (Laravel-style).
//!
//! Install after sessions. Mutating requests are checked for a matching token in
//! (order): `X-CSRF-TOKEN` → `X-XSRF-TOKEN` → query field →
//! `application/x-www-form-urlencoded` body field.
//! Multipart bodies are left to handlers ([`CsrfExt::verify_csrf`]) unless the
//! header/query carries the token.
//!
//! With [`Csrf::xsrf_cookie`] (default on), each response also gets a readable
//! `XSRF-TOKEN` cookie so SPA clients (axios) can send `X-XSRF-TOKEN`.

use http::Method;
use sova_cookies::{CookieBuilder, ResponseCookieExt};
use sova_core::extend::named;
use sova_core::{with_state, App, Error, Plugin, Request, Response};
use sova_session::SessionExt;
use std::collections::HashSet;
use std::sync::Arc;

const DEFAULT_SESSION_KEY: &str = "csrf";
const DEFAULT_FIELD: &str = "csrf";
const DEFAULT_HEADER: &str = "x-csrf-token";
const DEFAULT_XSRF_COOKIE: &str = "XSRF-TOKEN";
const DEFAULT_XSRF_HEADER: &str = "x-xsrf-token";

/// Per-request token placed by middleware / [`CsrfExt::csrf_token`].
#[derive(Clone, Debug)]
pub struct CsrfToken(pub String);

/// Session double-submit CSRF plugin.
#[derive(Clone)]
pub struct Csrf {
    session_key: String,
    field: String,
    field_explicit: bool,
    header: String,
    header_explicit: bool,
    /// Reject checked methods without a valid token (default: true).
    auto: bool,
    auto_explicit: bool,
    /// Paths excluded from auto-check (`/hook` exact, or `/api/*` prefix).
    except: Vec<String>,
    /// If non-empty, only these paths are checked (still minus `except`).
    only: Vec<String>,
    /// Methods that require a token. Empty → Laravel-style unsafe methods.
    methods: HashSet<Method>,
    xsrf_cookie: bool,
    xsrf_cookie_name: String,
    xsrf_header: String,
}

impl Csrf {
    pub fn new() -> Self {
        Self {
            session_key: DEFAULT_SESSION_KEY.into(),
            field: DEFAULT_FIELD.into(),
            field_explicit: false,
            header: DEFAULT_HEADER.into(),
            header_explicit: false,
            auto: true,
            auto_explicit: false,
            except: Vec::new(),
            only: Vec::new(),
            methods: HashSet::new(),
            xsrf_cookie: true,
            xsrf_cookie_name: DEFAULT_XSRF_COOKIE.into(),
            xsrf_header: DEFAULT_XSRF_HEADER.into(),
        }
    }

    pub fn session_key(mut self, key: impl Into<String>) -> Self {
        self.session_key = key.into();
        self
    }

    /// Form / JSON field name (default `csrf`).
    pub fn field(mut self, name: impl Into<String>) -> Self {
        self.field = name.into();
        self.field_explicit = true;
        self
    }

    /// Primary request header name (default `x-csrf-token`).
    pub fn header(mut self, name: impl Into<String>) -> Self {
        self.header = name.into();
        self.header_explicit = true;
        self
    }

    /// Disable automatic checks; use [`CsrfExt::verify_csrf`] in handlers.
    pub fn auto(mut self, on: bool) -> Self {
        self.auto = on;
        self.auto_explicit = true;
        self
    }

    /// Skip auto-check for a path (`/hook` exact, or `/api/*` prefix).
    pub fn skip(mut self, path: impl Into<String>) -> Self {
        self.except.push(path.into());
        self
    }

    /// Laravel-style alias for [`Self::skip`].
    pub fn except(mut self, path: impl Into<String>) -> Self {
        self.except.push(path.into());
        self
    }

    /// Only auto-check these paths (exact or `prefix*`). Empty = all paths (minus except).
    pub fn only(mut self, path: impl Into<String>) -> Self {
        self.only.push(path.into());
        self
    }

    /// Restrict which HTTP methods require a token.
    ///
    /// Default (empty set): all methods except GET, HEAD, OPTIONS, TRACE.
    pub fn methods(mut self, methods: impl IntoIterator<Item = Method>) -> Self {
        self.methods = methods.into_iter().collect();
        self
    }

    /// Set readable `XSRF-TOKEN` cookie on responses (default: true).
    pub fn xsrf_cookie(mut self, on: bool) -> Self {
        self.xsrf_cookie = on;
        self
    }

    pub fn xsrf_cookie_name(mut self, name: impl Into<String>) -> Self {
        self.xsrf_cookie_name = name.into();
        self
    }

    /// Alternate header read from the XSRF cookie (default `x-xsrf-token`).
    pub fn xsrf_header(mut self, name: impl Into<String>) -> Self {
        self.xsrf_header = name.into();
        self
    }
}

impl Default for Csrf {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Csrf {
    fn id(&self) -> &'static str {
        "csrf"
    }

    fn requires(&self) -> &'static [&'static str] {
        &["session"]
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("CSRF")
            .description("Session double-submit CSRF (Laravel-style except/XSRF cookie)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("csrf") {
                if !self.field_explicit {
                    if let Some(v) = section.get("field").and_then(|v| v.as_str()) {
                        self.field = v.to_string();
                    }
                }
                if !self.header_explicit {
                    if let Some(v) = section.get("header").and_then(|v| v.as_str()) {
                        self.header = v.to_string();
                    }
                }
                if !self.auto_explicit {
                    if let Some(v) = section.get("auto").and_then(|v| v.as_bool()) {
                        self.auto = v;
                    }
                }
            }
        }
        let cfg = Arc::new(self);
        app.use_middleware(named(
            "csrf",
            with_state(cfg, |cfg, mut req, next| async move {
                let token = ensure_token(&req, &cfg.session_key);
                req.set(CsrfToken(token.clone()));

                if cfg.auto
                    && method_checked(&req.method, &cfg.methods)
                    && path_checked(&req.path, cfg.as_ref())
                {
                    match submitted_token(&mut req, cfg.as_ref()).await {
                        Ok(TokenFind::Match(got)) if tokens_equal(&got, &token) => {}
                        Ok(TokenFind::Match(_)) => {
                            return Error::custom(403, "csrf mismatch").into_response();
                        }
                        Ok(TokenFind::Missing) => {
                            return Error::custom(403, "missing csrf token").into_response();
                        }
                        Ok(TokenFind::Deferred) => {
                            // multipart without header — handler must `verify_csrf`
                        }
                        Err(err) => return err.into_response(),
                    }
                }

                let mut res = next(req).await;
                if cfg.xsrf_cookie {
                    res = attach_xsrf_cookie(res, &cfg.xsrf_cookie_name, &token);
                }
                res
            }),
        ));
    }
}

fn attach_xsrf_cookie(res: Response, name: &str, token: &str) -> Response {
    let mut builder = CookieBuilder::build((name.to_string(), token.to_string())).path("/");
    if xsrf_cookie_secure() {
        builder = builder.secure(true);
    }
    res.cookie(builder.build())
}

fn xsrf_cookie_secure() -> bool {
    let production = std::env::var("SOVA_ENV")
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false);
    let forced = std::env::var("SESSION_SECURE")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(false);
    production || forced
}

/// Request helpers for CSRF tokens.
pub trait CsrfExt {
    /// Session CSRF token (creates one if missing).
    fn csrf_token(&self) -> String;

    /// Compare submitted value to the session token.
    fn verify_csrf(&self, submitted: Option<&str>) -> sova_core::Result<()>;
}

impl CsrfExt for Request {
    fn csrf_token(&self) -> String {
        if let Some(t) = self.get::<CsrfToken>() {
            return t.0.clone();
        }
        ensure_token(self, DEFAULT_SESSION_KEY)
    }

    fn verify_csrf(&self, submitted: Option<&str>) -> sova_core::Result<()> {
        let Some(got) = submitted.filter(|s| !s.is_empty()) else {
            return Err(Error::BadRequest("missing csrf token".into()));
        };
        let expected = self.csrf_token();
        if !tokens_equal(got, &expected) {
            return Err(Error::BadRequest("csrf mismatch".into()));
        }
        Ok(())
    }
}

fn ensure_token(req: &Request, session_key: &str) -> String {
    if let Some(t) = req.session().get(session_key) {
        if !t.is_empty() {
            return t;
        }
    }
    let t = new_token();
    req.session().set(session_key, t.clone());
    t
}

fn method_checked(method: &Method, configured: &HashSet<Method>) -> bool {
    if configured.is_empty() {
        return !matches!(
            *method,
            Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
        );
    }
    configured.contains(method)
}

fn path_matches(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| {
        if let Some(prefix) = p.strip_suffix('*') {
            path.starts_with(prefix)
        } else {
            path == p
        }
    })
}

fn path_checked(path: &str, cfg: &Csrf) -> bool {
    if path_matches(path, &cfg.except) {
        return false;
    }
    if cfg.only.is_empty() {
        return true;
    }
    path_matches(path, &cfg.only)
}

enum TokenFind {
    Match(String),
    Missing,
    /// Multipart without header/query — leave check to the handler.
    Deferred,
}

async fn submitted_token(req: &mut Request, cfg: &Csrf) -> sova_core::Result<TokenFind> {
    if let Some(h) = req.header(&cfg.header).filter(|s| !s.is_empty()) {
        return Ok(TokenFind::Match(h.to_string()));
    }
    if let Some(h) = req.header(&cfg.xsrf_header).filter(|s| !s.is_empty()) {
        // Axios may send URL-encoded cookie value.
        let decoded = urlencoding_decode(h);
        return Ok(TokenFind::Match(decoded));
    }
    if let Some(q) = req.query(&cfg.field).filter(|s| !s.is_empty()) {
        return Ok(TokenFind::Match(q.to_string()));
    }

    let ct = req.content_type().unwrap_or("");
    if ct.eq_ignore_ascii_case("application/x-www-form-urlencoded") {
        let bytes = req.body().await?;
        if let Ok(map) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&bytes) {
            if let Some((_, v)) = map.into_iter().find(|(k, _)| k == &cfg.field) {
                if !v.is_empty() {
                    return Ok(TokenFind::Match(v));
                }
            }
        }
        return Ok(TokenFind::Missing);
    }

    if ct.starts_with("multipart/") {
        return Ok(TokenFind::Deferred);
    }

    Ok(TokenFind::Missing)
}

fn urlencoding_decode(s: &str) -> String {
    // Minimal decode for axios XSRF values; leave as-is if not encoded.
    match percent_decode(s) {
        Some(v) => v,
        None => s.to_string(),
    }
}

fn percent_decode(s: &str) -> Option<String> {
    if !s.contains('%') {
        return None;
    }
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = from_hex(bytes[i + 1])?;
            let l = from_hex(bytes[i + 2])?;
            out.push((h << 4) | l);
            i += 3;
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn tokens_equal(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

fn new_token() -> String {
    let mut buf = [0u8; 16];
    if getrandom::getrandom(&mut buf).is_err() {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        return format!("{t:032x}");
    }
    let mut s = String::with_capacity(32);
    for b in buf {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use sova_core::Response;
    use sova_session::memory_sessions;

    fn app_with_csrf() -> App {
        let mut app = App::new();
        app.install(memory_sessions());
        app.install(Csrf::new());
        app.get("/", |req: Request| async move {
            Response::text(req.csrf_token())
        });
        app.post("/echo", |_req: Request| async { Response::text("ok") });
        app.put("/echo", |_req: Request| async { Response::text("ok") });
        app
    }

    fn session_cookie(res: &Response) -> String {
        res.headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find(|c| c.starts_with("sova_sid="))
            .map(|c| c.split(';').next().unwrap().to_string())
            .expect("session cookie")
    }

    fn cookie_named(res: &Response, name: &str) -> Option<String> {
        let prefix = format!("{name}=");
        res.headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find(|c| c.starts_with(&prefix))
            .map(|c| {
                c.split(';')
                    .next()
                    .unwrap()
                    .trim_start_matches(&prefix)
                    .to_string()
            })
    }

    #[tokio::test]
    async fn get_sets_token_and_xsrf_cookie() {
        let app = app_with_csrf();
        let res = app
            .handle(Request::builder().method(Method::GET).path("/").build())
            .await;
        assert_eq!(res.status_code().as_u16(), 200);
        let body = String::from_utf8_lossy(res.body_bytes().unwrap()).into_owned();
        assert!(!body.is_empty());
        assert_eq!(cookie_named(&res, "XSRF-TOKEN").as_deref(), Some(body.as_str()));
    }

    #[tokio::test]
    async fn post_without_token_is_403() {
        let app = app_with_csrf();
        let get = app
            .handle(Request::builder().method(Method::GET).path("/").build())
            .await;
        let cookie = session_cookie(&get);

        let res = app
            .handle(
                Request::builder()
                    .method(Method::POST)
                    .path("/echo")
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body("x=1")
                    .build(),
            )
            .await;
        assert_eq!(res.status_code().as_u16(), 403);
    }

    #[tokio::test]
    async fn post_with_form_token_ok() {
        let app = app_with_csrf();
        let get = app
            .handle(Request::builder().method(Method::GET).path("/").build())
            .await;
        let cookie = session_cookie(&get);
        let get2 = app
            .handle(
                Request::builder()
                    .method(Method::GET)
                    .path("/")
                    .header("cookie", &cookie)
                    .build(),
            )
            .await;
        let token = String::from_utf8_lossy(get2.body_bytes().unwrap()).into_owned();

        let res = app
            .handle(
                Request::builder()
                    .method(Method::POST)
                    .path("/echo")
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(format!("csrf={token}"))
                    .build(),
            )
            .await;
        assert_eq!(res.status_code().as_u16(), 200);
    }

    #[tokio::test]
    async fn post_with_xsrf_header_ok() {
        let app = app_with_csrf();
        let get = app
            .handle(Request::builder().method(Method::GET).path("/").build())
            .await;
        let cookie = session_cookie(&get);
        let token = cookie_named(&get, "XSRF-TOKEN").expect("xsrf");

        let res = app
            .handle(
                Request::builder()
                    .method(Method::POST)
                    .path("/echo")
                    .header("cookie", &cookie)
                    .header("x-xsrf-token", &token)
                    .header("content-type", "application/json")
                    .body("{}")
                    .build(),
            )
            .await;
        assert_eq!(res.status_code().as_u16(), 200);
    }

    #[tokio::test]
    async fn except_skips_path() {
        let mut app = App::new();
        app.install(memory_sessions());
        app.install(Csrf::new().except("/echo"));
        app.post("/echo", |_req: Request| async { Response::text("ok") });

        let res = app
            .handle(
                Request::builder()
                    .method(Method::POST)
                    .path("/echo")
                    .header("content-type", "application/json")
                    .body("{}")
                    .build(),
            )
            .await;
        assert_eq!(res.status_code().as_u16(), 200);
    }

    #[tokio::test]
    async fn only_limits_paths() {
        let mut app = App::new();
        app.install(memory_sessions());
        app.install(Csrf::new().only("/secure/*"));
        app.post("/open", |_req: Request| async { Response::text("ok") });
        app.post("/secure/x", |_req: Request| async { Response::text("ok") });

        let open = app
            .handle(
                Request::builder()
                    .method(Method::POST)
                    .path("/open")
                    .header("content-type", "application/json")
                    .body("{}")
                    .build(),
            )
            .await;
        assert_eq!(open.status_code().as_u16(), 200);

        let locked = app
            .handle(
                Request::builder()
                    .method(Method::POST)
                    .path("/secure/x")
                    .header("content-type", "application/json")
                    .body("{}")
                    .build(),
            )
            .await;
        assert_eq!(locked.status_code().as_u16(), 403);
    }

    #[tokio::test]
    async fn methods_filter() {
        let mut app = App::new();
        app.install(memory_sessions());
        app.install(Csrf::new().methods([Method::DELETE]));
        app.post("/echo", |_req: Request| async { Response::text("ok") });
        app.delete("/echo", |_req: Request| async { Response::text("ok") });

        let post = app
            .handle(
                Request::builder()
                    .method(Method::POST)
                    .path("/echo")
                    .header("content-type", "application/json")
                    .body("{}")
                    .build(),
            )
            .await;
        assert_eq!(post.status_code().as_u16(), 200);

        let del = app
            .handle(
                Request::builder()
                    .method(Method::DELETE)
                    .path("/echo")
                    .header("content-type", "application/json")
                    .body("{}")
                    .build(),
            )
            .await;
        assert_eq!(del.status_code().as_u16(), 403);
    }
}
