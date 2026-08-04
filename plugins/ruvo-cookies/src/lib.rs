//! Cookie parsing middleware and `Response::cookie` extension.

use cookie::Cookie;
use http::{header, HeaderMap, HeaderValue};
use ruvo_core::extend::named;
use ruvo_core::{App, Next, Plugin, Request, Response};
use std::collections::HashMap;

/// Parsed request cookies stored in extensions.
#[derive(Debug, Default, Clone)]
pub struct Cookies {
    map: HashMap<String, String>,
}

impl Cookies {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.map.get(name).map(|s| s.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.map.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// Parse `Cookie` header into [`Cookies`] on each request.
pub struct CookieLayer;

impl Plugin for CookieLayer {
    fn install(self, app: &mut App) {
        app.state(CookieLayerPresent);
        app.use_middleware(named("cookies", |mut req: Request, next: Next| async move {
            let parsed = parse_cookies(&req.headers);
            req.set(parsed);
            next(req).await
        }));
    }
}

/// Marker inserted by [`CookieLayer`] so other plugins can require cookies at startup.
#[derive(Clone, Copy, Debug)]
pub struct CookieLayerPresent;

fn parse_cookies(headers: &HeaderMap) -> Cookies {
    let mut cookies = Cookies::default();
    if let Some(raw) = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
    {
        for part in raw.split(';') {
            if let Ok(c) = Cookie::parse(part.trim()) {
                cookies
                    .map
                    .insert(c.name().to_string(), c.value().to_string());
            }
        }
    }
    cookies
}

/// Attach `Set-Cookie` on a [`Response`] (lives outside core on purpose).
pub trait ResponseCookieExt {
    fn cookie(self, cookie: Cookie<'_>) -> Self;
}

impl ResponseCookieExt for Response {
    fn cookie(mut self, cookie: Cookie<'_>) -> Self {
        let value = cookie.encoded().to_string();
        if let Ok(v) = HeaderValue::from_str(&value) {
            self.headers_mut().append(header::SET_COOKIE, v);
        }
        self
    }
}

/// Re-export cookie builder for responses.
pub use cookie::Cookie as CookieBuilder;
