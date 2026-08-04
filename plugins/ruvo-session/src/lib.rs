//! Cookie-backed sessions for Ruvo.

use cookie::{Cookie, SameSite};
use ruvo_cookies::{CookieLayer, Cookies, ResponseCookieExt};
use ruvo_core::extend::named;
use ruvo_core::{with_state, App, Plugin};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Pluggable session backend.
pub trait SessionStore: Send + Sync + 'static {
    fn get(&self, id: &str) -> Option<HashMap<String, String>>;
    fn set(&self, id: &str, data: HashMap<String, String>);
    fn remove(&self, id: &str);
}

type SessionEntry = (HashMap<String, String>, Instant);

/// In-memory sessions with TTL.
#[derive(Clone)]
pub struct MemoryStore {
    inner: Arc<Mutex<HashMap<String, SessionEntry>>>,
    ttl: Duration,
    max_entries: usize,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::from_secs(86400),
            max_entries: 10_000,
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn max_entries(mut self, n: usize) -> Self {
        self.max_entries = n.max(1);
        self
    }

    fn sweep(map: &mut HashMap<String, (HashMap<String, String>, Instant)>, ttl: Duration) {
        map.retain(|_, (_, at)| at.elapsed() < ttl);
    }
}

impl SessionStore for MemoryStore {
    fn get(&self, id: &str) -> Option<HashMap<String, String>> {
        let mut map = self.inner.lock().unwrap();
        if map.len() > self.max_entries {
            Self::sweep(&mut map, self.ttl);
        }
        if let Some((data, at)) = map.get(id) {
            if at.elapsed() < self.ttl {
                return Some(data.clone());
            }
            map.remove(id);
        }
        None
    }

    fn set(&self, id: &str, data: HashMap<String, String>) {
        let mut map = self.inner.lock().unwrap();
        if map.len() >= self.max_entries {
            Self::sweep(&mut map, self.ttl);
            while map.len() >= self.max_entries {
                if let Some(k) = map.keys().next().cloned() {
                    map.remove(&k);
                } else {
                    break;
                }
            }
        }
        map.insert(id.to_string(), (data, Instant::now()));
    }

    fn remove(&self, id: &str) {
        self.inner.lock().unwrap().remove(id);
    }
}

/// No-op store (second implementation justifying the trait seam).
#[derive(Clone, Default)]
pub struct NullStore;

impl SessionStore for NullStore {
    fn get(&self, _id: &str) -> Option<HashMap<String, String>> {
        None
    }
    fn set(&self, _id: &str, _data: HashMap<String, String>) {}
    fn remove(&self, _id: &str) {}
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

    /// Like [`get`](Self::get), or `default` when the key is missing.
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
}

/// Session middleware plugin.
pub struct SessionLayer<S: SessionStore> {
    store: Arc<S>,
    cookie_name: String,
    secure: bool,
}

impl<S: SessionStore> SessionLayer<S> {
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(store),
            cookie_name: "ruvo_sid".into(),
            secure: false,
        }
    }

    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }

    /// Set the `Secure` flag on the session cookie (HTTPS only).
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
}

impl<S: SessionStore> Plugin for SessionLayer<S> {
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

                let data = layer.store.get(&sid).unwrap_or_default();
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
                    layer.store.set(&id, data);
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

pub fn memory_sessions() -> SessionLayer<MemoryStore> {
    SessionLayer::new(MemoryStore::new())
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

impl Session {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_entries_evicts() {
        let store = MemoryStore::new().max_entries(2);
        store.set("a", HashMap::from([("k".into(), "1".into())]));
        store.set("b", HashMap::from([("k".into(), "2".into())]));
        store.set("c", HashMap::from([("k".into(), "3".into())]));
        let map = store.inner.lock().unwrap();
        assert!(map.len() <= 2);
        assert!(map.contains_key("c"));
    }
}
