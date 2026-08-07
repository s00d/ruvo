//! Cookie-backed sessions for Ruvo (Express [`express-session`](https://expressjs.com/en/resources/middleware/session.html)-style).

use bytes::Bytes;
use cookie::Cookie;
pub use cookie::SameSite;
use ruvo_cookies::{CookieLayer, CookieLayerPresent, Cookies, ResponseCookieExt};
use ruvo_core::extend::{named, BoxFuture, Needs};
use ruvo_core::{with_state, App, Error, Plugin, Request, Result};
use ruvo_store::{namespace, KvStore, MemoryStore as KvMemory};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn encode(data: &HashMap<String, String>) -> Bytes {
    let mut out = String::new();
    for (k, v) in data {
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
    /// Destroyed — remove from store and clear cookie.
    destroyed: bool,
    /// Sid rotated; old id should be removed after save.
    old_id: Option<String>,
    /// Fresh sid not yet persisted (for save_uninitialized).
    is_new: bool,
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

    pub fn remove(&self, key: &str) {
        let mut g = self.inner.lock().unwrap();
        if g.data.remove(key).is_some() {
            g.dirty = true;
        }
    }

    pub fn clear(&self) {
        let mut g = self.inner.lock().unwrap();
        if !g.data.is_empty() {
            g.data.clear();
            g.dirty = true;
        }
    }

    /// Drop session data and mark for store removal + cookie clear.
    pub fn destroy(&self) {
        let mut g = self.inner.lock().unwrap();
        g.data.clear();
        g.dirty = false;
        g.destroyed = true;
    }

    /// Issue a new session id, keep data, delete the old store entry.
    pub fn regenerate(&self) {
        let mut g = self.inner.lock().unwrap();
        let old = std::mem::replace(&mut g.id, new_sid());
        g.old_id = Some(old);
        g.dirty = true;
        g.is_new = true;
        g.destroyed = false;
    }

    fn snapshot(&self) -> SessionSnapshot {
        let g = self.inner.lock().unwrap();
        SessionSnapshot {
            id: g.id.clone(),
            data: g.data.clone(),
            dirty: g.dirty,
            destroyed: g.destroyed,
            old_id: g.old_id.clone(),
            is_new: g.is_new,
        }
    }

    /// Detached empty session (not backed by a store — writes are local only).
    fn empty() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionInner {
                id: String::new(),
                data: HashMap::new(),
                dirty: false,
                destroyed: false,
                old_id: None,
                is_new: true,
            })),
        }
    }
}

struct SessionSnapshot {
    id: String,
    data: HashMap<String, String>,
    dirty: bool,
    destroyed: bool,
    old_id: Option<String>,
    is_new: bool,
}

type HookFn = Arc<dyn Fn(Session, Request) -> BoxFuture<Result<Request>> + Send + Sync>;

/// Session middleware over a [`KvStore`] (typically `namespace(store, "sess")`).
///
/// Cookie load/save stays here. App logic (hydrate user, ACL, …) goes in [`SessionLayer::hook`].
pub struct SessionLayer {
    store: Arc<dyn KvStore>,
    cookie_name: String,
    secure: bool,
    http_only: bool,
    same_site: SameSite,
    path: String,
    ttl: Duration,
    rolling: bool,
    save_uninitialized: bool,
    hook: Option<HookFn>,
}

impl SessionLayer {
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self {
            store,
            cookie_name: "ruvo_sid".into(),
            secure: false,
            http_only: true,
            same_site: SameSite::Lax,
            path: "/".into(),
            ttl: Duration::from_secs(86400),
            rolling: false,
            save_uninitialized: false,
            hook: None,
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

    pub fn http_only(mut self, on: bool) -> Self {
        self.http_only = on;
        self
    }

    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Refresh store TTL and Set-Cookie on every response.
    pub fn rolling(mut self, on: bool) -> Self {
        self.rolling = on;
        self
    }

    /// Persist empty new sessions (Express default false).
    pub fn save_uninitialized(mut self, on: bool) -> Self {
        self.save_uninitialized = on;
        self
    }

    /// After cookie session is loaded (and set on `req`), run app logic.
    ///
    /// Clone Arcs from `req` **before** `async move`. Return the (possibly mutated) request:
    ///
    /// ```ignore
    /// SessionLayer::new(store).hook(|sess, mut req| async move {
    ///     if let Some(id) = sess.get("user_id") {
    ///         let db = req.state::<Db>().clone();
    ///         if let Some(u) = find_user(&db, &id).await? {
    ///             req.set(u);
    ///         }
    ///     }
    ///     Ok(req)
    /// })
    /// ```
    pub fn hook<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Session, Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Request>> + Send + 'static,
    {
        self.hook = Some(Arc::new(move |sess, req| Box::pin(f(sess, req))));
        self
    }
}

impl Plugin for SessionLayer {
    fn id(&self) -> &'static str {
        "session"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Session")
            .description("Cookie sessions backed by a KvStore")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        if !app.has_plugin("cookies") {
            app.install(CookieLayer);
        }

        let store_check = Arc::clone(&self.store);
        app.register_check("kv", move |_state| {
            let store = Arc::clone(&store_check);
            async move {
                let probe = "__ruvo_check__";
                store
                    .set(probe, Bytes::from_static(b"1"), Some(Duration::from_secs(5)))
                    .await;
                let got = store.get(probe).await;
                store.remove(probe).await;
                if got.is_none() {
                    return Err(Error::Internal("kv store probe failed".into()));
                }
                Ok(())
            }
        });

        app.use_middleware(named(
            "session",
            with_state(self, |layer, mut req, next| {
                async move {
                    let had_cookie = req
                        .get::<Cookies>()
                        .and_then(|c| c.get(&layer.cookie_name).map(|s| s.to_string()));
                    let is_new = had_cookie.is_none();
                    let sid = had_cookie.unwrap_or_else(new_sid);

                    let data = if is_new {
                        HashMap::new()
                    } else {
                        layer
                            .store
                            .get(&sid)
                            .await
                            .map(|b| decode(&b))
                            .unwrap_or_default()
                    };
                    let session = Session {
                        inner: Arc::new(Mutex::new(SessionInner {
                            id: sid,
                            data,
                            dirty: false,
                            destroyed: false,
                            old_id: None,
                            is_new,
                        })),
                    };
                    req.set(session.clone());

                    let req = if let Some(hook) = &layer.hook {
                        match hook(session.clone(), req).await {
                            Ok(r) => r,
                            Err(err) => return err.into_response(),
                        }
                    } else {
                        req
                    };

                    let mut res = next(req).await;
                    let snap = session.snapshot();

                    if snap.destroyed {
                        layer.store.remove(&snap.id).await;
                        if let Some(old) = &snap.old_id {
                            layer.store.remove(old).await;
                        }
                        // Expire cookie.
                        let mut builder = Cookie::build((layer.cookie_name.clone(), ""))
                            .http_only(layer.http_only)
                            .same_site(layer.same_site)
                            .path(layer.path.clone())
                            .max_age(cookie::time::Duration::seconds(0));
                        if layer.secure {
                            builder = builder.secure(true);
                        }
                        return res.cookie(builder.build());
                    }

                    let should_persist = snap.dirty
                        || (snap.is_new && layer.save_uninitialized)
                        || (layer.rolling && !snap.is_new);

                    if should_persist {
                        if let Some(old) = &snap.old_id {
                            layer.store.remove(old).await;
                        }
                        layer
                            .store
                            .set(&snap.id, encode(&snap.data), Some(layer.ttl))
                            .await;
                    }

                    let should_set_cookie = should_persist || (layer.rolling && !snap.is_new);
                    // Also set cookie when new + dirty (already in should_persist).
                    if should_set_cookie || (snap.is_new && snap.dirty) {
                        let mut builder = Cookie::build((layer.cookie_name.clone(), snap.id))
                            .http_only(layer.http_only)
                            .same_site(layer.same_site)
                            .path(layer.path.clone());
                        if layer.secure {
                            builder = builder.secure(true);
                        }
                        // Max-Age from ttl when rolling or first persist.
                        if let Ok(secs) = i64::try_from(layer.ttl.as_secs()) {
                            builder = builder.max_age(cookie::time::Duration::seconds(secs));
                        }
                        res = res.cookie(builder.build());
                    }

                    res
                }
            }),
        ));
        app.with(Needs::<CookieLayerPresent>::new());
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
