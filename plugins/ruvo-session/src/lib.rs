//! Cookie-backed sessions for Ruvo (storage via [`ruvo_store::KvStore`]).

use bytes::Bytes;
use cookie::{Cookie, SameSite};
use ruvo_cookies::{CookieLayer, Cookies, ResponseCookieExt};
use ruvo_core::extend::named;
use ruvo_core::{with_state, App, Plugin};
use ruvo_store::{namespace, KvStore, MemoryStore as KvMemory};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn encode(data: &HashMap<String, String>) -> Bytes {
    let mut out = String::new();
    for (k, v) in data {
        // simple length-prefixed-ish: key\0value\n
        out.push_str(k);
        out.push('\0');
        out.push_str(v);
        out.push('\n');
    }
    Bytes::from(out)
}

fn decode(bytes: &[u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in std::str::from_utf8(bytes).unwrap_or("").lines() {
        if let Some((k, v)) = line.split_once('\0') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

#[derive(Debug)]
struct SessionInner {
    id: String,
    data: HashMap<String, String>,
    dirty: bool,
}

/// Shared per-request session (cheap to clone into middleware after `next`).
#[derive(Clone, Debug)]
pub struct Session {
    inner: Arc<Mutex<SessionInner>>,
}

impl Session {
    pub fn id(&self) -> String {
        self.inner.lock().unwrap().id.clone()
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.inner.lock().unwrap().data.get(key).cloned()
    }

    pub fn get_or(&self, key: &str, default: impl Into<String>) -> String {
        self.get(key).unwrap_or_else(|| default.into())
    }

    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut g = self.inner.lock().unwrap();
        g.data.insert(key.into(), value.into());
        g.dirty = true;
    }

    fn snapshot(&self) -> (String, HashMap<String, String>, bool) {
        let g = self.inner.lock().unwrap();
        (g.id.clone(), g.data.clone(), g.dirty)
    }

    /// Detached empty session (not backed by a store — writes are local only).
    fn empty() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionInner {
                id: String::new(),
                data: HashMap::new(),
                dirty: false,
            })),
        }
    }
}

/// Session middleware over a [`KvStore`] (typically `namespace(store, "sess")`).
pub struct SessionLayer {
    store: Arc<dyn KvStore>,
    cookie_name: String,
    secure: bool,
    ttl: Duration,
}

impl SessionLayer {
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self {
            store,
            cookie_name: "ruvo_sid".into(),
            secure: false,
            ttl: Duration::from_secs(86400),
        }
    }

    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }
}

impl Plugin for SessionLayer {
    fn install(self, app: &mut App) {
        CookieLayer.install(app);
        app.use_middleware(named(
            "session",
            with_state(self, |layer, mut req, next| {
                async move {
                    let sid = req
                        .get::<Cookies>()
                        .and_then(|c| c.get(&layer.cookie_name).map(|s| s.to_string()))
                        .unwrap_or_else(new_sid);

                    let data = layer
                        .store
                        .get(&sid)
                        .await
                        .map(|b| decode(&b))
                        .unwrap_or_default();
                    let session = Session {
                        inner: Arc::new(Mutex::new(SessionInner {
                            id: sid,
                            data,
                            dirty: false,
                        })),
                    };
                    req.set(session.clone());

                    let mut res = next(req).await;
                    let (id, data, dirty) = session.snapshot();
                    if dirty {
                        layer.store.set(&id, encode(&data), Some(layer.ttl)).await;
                    }
                    let mut builder = Cookie::build((layer.cookie_name.clone(), id))
                        .http_only(true)
                        .same_site(SameSite::Lax)
                        .path("/");
                    if layer.secure {
                        builder = builder.secure(true);
                    }
                    res = res.cookie(builder.build());
                    res
                }
            }),
        ));
    }
}

fn new_sid() -> String {
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

/// Default in-memory sessions (`ruvo_store::MemoryStore` under `sess:`).
pub fn memory_sessions() -> SessionLayer {
    SessionLayer::new(Arc::new(namespace(Arc::new(KvMemory::new()), "sess")))
}

/// Convenient access to the request [`Session`] (empty session if the layer is absent).
pub trait SessionExt {
    fn session(&self) -> Session;
}

impl SessionExt for ruvo_core::Request {
    fn session(&self) -> Session {
        self.get::<Session>().cloned().unwrap_or_else(Session::empty)
    }
}
