//! Cookie-backed sessions for Sova (Express [`express-session`](https://expressjs.com/en/resources/middleware/session.html)-style).
//!
//! Flash helpers ([`Session::flash`], [`Session::take`]) store one-shot values for the next
//! request (status messages, validation errors, old form input).
//!
//! Persistence is [`SessionStore`]: [`KvSessionStore`], [`SqlSessionStore`], or
//! [`RedisSessionStore`]. Logout others/all:
//! [`SessionExt::logout_other_sessions`] / [`SessionExt::logout_all_sessions`].

mod store;
#[cfg(feature = "sql")]
mod sql;
#[cfg(feature = "redis")]
mod redis_store;

use cookie::Cookie;
pub use cookie::SameSite;
use sova_cookies::{CookieLayer, CookieLayerPresent, Cookies, ResponseCookieExt};
use sova_core::extend::{named, BoxFuture, Needs};
use sova_core::{with_state, App, Error, Plugin, Request, Result};
use sova_store::{namespace, KvStore, MemoryStore as KvMemory};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use store::{KvSessionStore, SessionStore, SessionStoreHandle, SESSION_USER_KEY};
#[cfg(feature = "sql")]
pub use sql::SqlSessionStore;
#[cfg(feature = "redis")]
pub use redis_store::RedisSessionStore;

/// Session key for a one-line success/status message (templates: `status`).
pub const FLASH_STATUS: &str = "flash_status";
/// Session key for validation error map JSON (templates: `errors`).
pub const FLASH_ERRORS: &str = "flash_errors";
/// Session key for old form input JSON (templates: `old`).
pub const FLASH_OLD: &str = "flash_old";

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

    /// One-shot value: survives until the next request that [`Self::take`]s it.
    pub fn flash(&self, key: impl Into<String>, value: impl Into<String>) {
        self.set(key, value);
    }

    /// Convenience for [`FLASH_STATUS`].
    pub fn flash_status(&self, msg: impl Into<String>) {
        self.flash(FLASH_STATUS, msg);
    }

    /// JSON flash (errors / old maps).
    pub fn flash_json(&self, key: &str, value: &serde_json::Value) {
        self.flash(
            key,
            serde_json::to_string(value).unwrap_or_else(|_| "{}".into()),
        );
    }

    /// Flash validation-style errors map (`{ field: message }`).
    pub fn flash_errors(&self, errors: &serde_json::Value) {
        self.flash_json(FLASH_ERRORS, errors);
    }

    /// Flash old form input map.
    pub fn flash_old(&self, old: &serde_json::Value) {
        self.flash_json(FLASH_OLD, old);
    }

    /// Read and clear a session key (empty string if missing).
    pub fn take(&self, key: &str) -> String {
        let mut g = self.inner.lock().unwrap();
        let val = g.data.remove(key).unwrap_or_default();
        if !val.is_empty() {
            g.dirty = true;
        }
        val
    }

    /// Parse JSON from [`Self::take`], or `{}` on empty/invalid.
    pub fn take_json(&self, key: &str) -> serde_json::Value {
        let raw = self.take(key);
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
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

    /// Replace the entire session bag (marks dirty).
    pub fn replace(&self, data: HashMap<String, String>) {
        let mut g = self.inner.lock().unwrap();
        g.data = data;
        g.dirty = true;
    }

    /// Snapshot of all key/value pairs.
    pub fn data(&self) -> HashMap<String, String> {
        self.inner.lock().unwrap().data.clone()
    }

    /// Bind this session to a user id (for logout-others/all via [`SessionStore`]).
    pub fn bind_user(&self, user_id: impl Into<String>) {
        self.set(SESSION_USER_KEY, user_id);
    }

    /// User id from [`Self::bind_user`], if any.
    pub fn user_id(&self) -> Option<String> {
        self.get(SESSION_USER_KEY).filter(|s| !s.is_empty())
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

/// Session middleware over a [`SessionStore`].
///
/// Cookie load/save stays here. App logic (hydrate user, ACL, …) goes in [`SessionLayer::hook`].
pub struct SessionLayer {
    store: Arc<dyn SessionStore>,
    cookie_name: String,
    cookie_name_explicit: bool,
    secure: bool,
    http_only: bool,
    same_site: SameSite,
    same_site_explicit: bool,
    path: String,
    ttl: Duration,
    ttl_explicit: bool,
    rolling: bool,
    save_uninitialized: bool,
    hook: Option<HookFn>,
}

impl SessionLayer {
    /// Wrap a [`KvStore`] (typically `namespace(store, "sess")`) in [`KvSessionStore`].
    pub fn new(kv: Arc<dyn KvStore>) -> Self {
        Self::from_store(Arc::new(KvSessionStore::new(kv)))
    }

    /// Use an arbitrary [`SessionStore`] (e.g. [`SqlSessionStore`]).
    pub fn from_store(store: Arc<dyn SessionStore>) -> Self {
        Self {
            store,
            cookie_name: "sova_sid".into(),
            cookie_name_explicit: false,
            secure: false,
            http_only: true,
            same_site: SameSite::Lax,
            same_site_explicit: false,
            path: "/".into(),
            ttl: Duration::from_secs(86400),
            ttl_explicit: false,
            rolling: false,
            save_uninitialized: false,
            hook: None,
        }
    }

    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self.cookie_name_explicit = true;
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
        self.same_site_explicit = true;
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self.ttl_explicit = true;
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

    fn apply_config(&mut self, app: &App) {
        let Some(doc) = app.config_doc() else {
            return;
        };
        let Some(section) = doc.section("session") else {
            return;
        };
        if !self.cookie_name_explicit {
            if let Some(n) = section.get("cookie").and_then(|v| v.as_str()) {
                self.cookie_name = n.to_string();
            }
        }
        if !self.ttl_explicit {
            if let Some(s) = section.get("ttl").and_then(|v| v.as_str()) {
                if let Ok(d) = sova_core::extend::parse_duration(s) {
                    self.ttl = d;
                }
            }
        }
        if !self.same_site_explicit {
            if let Some(s) = section.get("same_site").and_then(|v| v.as_str()) {
                self.same_site = match s.to_ascii_lowercase().as_str() {
                    "strict" => SameSite::Strict,
                    "none" => SameSite::None,
                    _ => SameSite::Lax,
                };
            }
        }
        if let Some(v) = section.get("secure").and_then(|v| v.as_bool()) {
            self.secure = v;
        }
    }
}

impl Plugin for SessionLayer {
    fn id(&self) -> &'static str {
        "session"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Session")
            .description("Cookie sessions backed by a SessionStore")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        self.apply_config(app);
        if !app.has_plugin("cookies") {
            app.install(CookieLayer);
        }

        app.state(SessionStoreHandle(Arc::clone(&self.store)));

        let store_check = Arc::clone(&self.store);
        app.register_check("session", move |_state| {
            let store = Arc::clone(&store_check);
            async move {
                let probe = "__sova_session_check__";
                let mut data = HashMap::new();
                data.insert("_probe".into(), "1".into());
                store
                    .save(probe, &data, Duration::from_secs(5))
                    .await;
                let got = store.load(probe).await;
                store.destroy(probe).await;
                if got.is_none() {
                    return Err(Error::Internal("session store probe failed".into()));
                }
                Ok(())
            }
        });

        app.use_middleware(named(
            "session",
            with_state(self, move |layer, mut req, next| {
                async move {
                    let had_cookie = req
                        .get::<Cookies>()
                        .and_then(|c| c.get(&layer.cookie_name).map(|s| s.to_string()));
                    let is_new = had_cookie.is_none();
                    let sid = had_cookie.unwrap_or_else(new_sid);

                    let data = if is_new {
                        HashMap::new()
                    } else {
                        layer.store.load(&sid).await.unwrap_or_default()
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
                        layer.store.destroy(&snap.id).await;
                        if let Some(old) = &snap.old_id {
                            layer.store.destroy(old).await;
                        }
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
                            layer.store.destroy(old).await;
                        }
                        layer
                            .store
                            .save(&snap.id, &snap.data, layer.ttl)
                            .await;
                    }

                    let should_set_cookie = should_persist || (layer.rolling && !snap.is_new);
                    if should_set_cookie || (snap.is_new && snap.dirty) {
                        let mut builder = Cookie::build((layer.cookie_name.clone(), snap.id))
                            .http_only(layer.http_only)
                            .same_site(layer.same_site)
                            .path(layer.path.clone());
                        if layer.secure {
                            builder = builder.secure(true);
                        }
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

/// Default in-memory sessions (`sova_store::MemoryStore` under `sess:`).
pub fn memory_sessions() -> SessionLayer {
    SessionLayer::new(Arc::new(namespace(Arc::new(KvMemory::new()), "sess")))
}

/// Convenient access to the request [`Session`] (empty session if the layer is absent).
pub trait SessionExt {
    fn session(&self) -> Session;

    fn flash(&self, key: impl Into<String>, value: impl Into<String>) {
        self.session().flash(key, value);
    }

    fn flash_status(&self, msg: impl Into<String>) {
        self.session().flash_status(msg);
    }

    fn flash_errors(&self, errors: &serde_json::Value) {
        self.session().flash_errors(errors);
    }

    fn flash_old(&self, old: &serde_json::Value) {
        self.session().flash_old(old);
    }

    /// Invalidate every other session for the bound user; keep the current cookie.
    ///
    /// Requires [`Session::bind_user`] (done by Passport/Fortify login).
    fn logout_other_sessions(
        &self,
    ) -> impl std::future::Future<Output = Result<u64>> + Send;

    /// Invalidate all sessions for the bound user, including this one (clears cookie).
    fn logout_all_sessions(
        &mut self,
    ) -> impl std::future::Future<Output = Result<u64>> + Send;
}

impl SessionExt for sova_core::Request {
    fn session(&self) -> Session {
        self.get::<Session>().cloned().unwrap_or_else(Session::empty)
    }

    async fn logout_other_sessions(&self) -> Result<u64> {
        let sess = self.session();
        let uid = sess
            .user_id()
            .ok_or_else(|| Error::BadRequest("no bound session user".into()))?;
        let handle = self.try_state::<SessionStoreHandle>().ok_or_else(|| {
            Error::Internal("SessionStore missing (is SessionLayer installed?)".into())
        })?;
        Ok(handle
            .0
            .destroy_user(&uid, Some(&sess.id()))
            .await)
    }

    async fn logout_all_sessions(&mut self) -> Result<u64> {
        let sess = self.session();
        let uid = sess
            .user_id()
            .ok_or_else(|| Error::BadRequest("no bound session user".into()))?;
        let handle = self.try_state::<SessionStoreHandle>().ok_or_else(|| {
            Error::Internal("SessionStore missing (is SessionLayer installed?)".into())
        })?;
        let n = handle.0.destroy_user(&uid, None).await;
        sess.destroy();
        let _ = self.take::<sova_core::RateLimitIdentity>();
        Ok(n)
    }
}

#[cfg(test)]
mod flash_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flash_take_is_one_shot() {
        let s = Session::empty();
        s.flash_status("Saved");
        assert_eq!(s.take(FLASH_STATUS), "Saved");
        assert_eq!(s.take(FLASH_STATUS), "");

        s.flash_old(&json!({ "email": "a@b.c" }));
        let old = s.take_json(FLASH_OLD);
        assert_eq!(old["email"], "a@b.c");
        assert_eq!(s.take_json(FLASH_OLD), json!({}));
    }
}
