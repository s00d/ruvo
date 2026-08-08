//! Passport core: strategy registry, session serialize/deserialize, login/logout.

use ruvo_core::extend::{named, BoxFuture, MwEntry};
use ruvo_core::{with_state, App, Error, Plugin, RateLimitIdentity, Request, Result};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

const DEFAULT_SESSION_KEY: &str = "passport:user";

type SerializeFn = Arc<dyn Fn(&Request) -> BoxFuture<Result<Option<String>>> + Send + Sync>;
type DeserializeFn =
    Arc<dyn Fn(String, Request) -> BoxFuture<Result<Request>> + Send + Sync>;

/// Marker that a principal id was established (session and/or strategy).
#[derive(Clone, Debug)]
pub struct Authenticated {
    pub id: String,
}

/// Passport.js-style authentication manager.
///
/// ```ignore
/// app.install(
///   Passport::new()
///     .strategy("api-key", Auth::api_key("x-api-key", verify).middleware())
///     .serialize_user(|req| async { Ok(req.user::<User>().map(|u| u.id.to_string())) })
///     .deserialize_user(|id, mut req| async move {
///         req.set(load_user(&id).await?);
///         Ok(req)
///     }),
/// );
/// api.use_middleware(Passport::authenticate("api-key"));
/// ```
pub struct Passport {
    strategies: HashMap<String, MwEntry>,
    session_key: String,
    serialize: Option<SerializeFn>,
    deserialize: Option<DeserializeFn>,
}

#[derive(Clone)]
pub(crate) struct PassportState {
    pub(crate) session_key: String,
    serialize: Option<SerializeFn>,
    deserialize: Option<DeserializeFn>,
    strategies: Arc<HashMap<String, MwEntry>>,
}

impl Passport {
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
            session_key: DEFAULT_SESSION_KEY.into(),
            serialize: None,
            deserialize: None,
        }
    }

    pub fn session_key(mut self, key: impl Into<String>) -> Self {
        self.session_key = key.into();
        self
    }

    /// Register a named strategy (`passport.authenticate("name")`).
    pub fn strategy(mut self, name: impl Into<String>, mw: MwEntry) -> Self {
        self.strategies.insert(name.into(), mw);
        self
    }

    /// How to turn the current request user into a session id string.
    pub fn serialize_user<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(&Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<String>>> + Send + 'static,
    {
        self.serialize = Some(Arc::new(move |req| Box::pin(f(req))));
        self
    }

    /// Restore the user on each request from the session id.
    pub fn deserialize_user<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(String, Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Request>> + Send + 'static,
    {
        self.deserialize = Some(Arc::new(move |id, req| Box::pin(f(id, req))));
        self
    }

    /// Run a registered strategy (401 if missing/fail — unless strategy is optional).
    pub fn authenticate(name: &str) -> MwEntry {
        let name = name.to_string();
        named(
            format!("passport-authenticate:{name}"),
            with_state(name, |name, req, next| async move {
                let Some(state) = req.try_state::<PassportState>() else {
                    return Error::Internal("Passport is not installed".into()).into_response();
                };
                let Some(entry) = state.strategies.get(name.as_str()) else {
                    return Error::Internal(format!("unknown passport strategy `{name}`"))
                        .into_response();
                };
                let mw = Arc::clone(&entry.mw);
                mw(req, next).await
            }),
        )
    }
}

impl Default for Passport {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Passport {
    fn id(&self) -> &'static str {
        "passport"
    }

    fn requires(&self) -> &'static [&'static str] {
        if self.serialize.is_some() || self.deserialize.is_some() {
            &["session"]
        } else {
            &[]
        }
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Passport")
            .description("Passport-style authentication (strategies + session)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        let needs_session = self.serialize.is_some() || self.deserialize.is_some();
        #[cfg(not(feature = "session"))]
        if needs_session {
            app.on_startup(|_s| async {
                Err(Error::Internal(
                    "Passport serialize/deserialize requires feature `session` on ruvo-passport"
                        .into(),
                ))
            });
            return;
        }
        let _ = needs_session;

        let state = PassportState {
            session_key: self.session_key.clone(),
            serialize: self.serialize.clone(),
            deserialize: self.deserialize.clone(),
            strategies: Arc::new(self.strategies),
        };
        app.state(state.clone());

        #[cfg(feature = "session")]
        if state.deserialize.is_some() {
            app.use_middleware(named(
                "passport-session",
                with_state(state, |state, req, next| async move {
                    use ruvo_session::SessionExt;
                    let sid = req.session().get(&state.session_key);
                    if let (Some(id), Some(de)) = (sid, &state.deserialize) {
                        match de(id.clone(), req).await {
                            Ok(mut r) => {
                                r.set(Authenticated { id: id.clone() });
                                r.set(RateLimitIdentity(id));
                                return next(r).await;
                            }
                            Err(err) => return err.into_response(),
                        }
                    }
                    next(req).await
                }),
            ));
        }
    }
}

/// Passport request helpers (`login` / `logout` / `isAuthenticated` / `user`).
///
/// [`Self::login`] regenerates the session id (fixation protection) and stores the user id
/// under Passport's session key so the next request deserializes the same principal.
pub trait PassportExt {
    /// Establish a login: rotate session, set [`Authenticated`], typed user, persist id.
    fn login<U: Send + Sync + 'static>(&mut self, user_id: impl Into<String>, user: U);

    /// Persist id only (user already on request or will be set by deserialize next time).
    fn login_id(&mut self, user_id: impl Into<String>);

    /// Clear passport session id, rotate sid, drop [`Authenticated`] from this request.
    fn logout(&mut self);

    fn is_authenticated(&self) -> bool;

    fn user_id(&self) -> Option<String>;

    fn user<U: Send + Sync + 'static>(&self) -> Option<&U>;

    fn require_user<U: Send + Sync + 'static>(&self) -> Result<&U>;
}

impl PassportExt for Request {
    fn login<U: Send + Sync + 'static>(&mut self, user_id: impl Into<String>, user: U) {
        let id = user_id.into();
        rotate_session(self);
        self.set(Authenticated { id: id.clone() });
        self.set(RateLimitIdentity(id.clone()));
        self.set(user);
        persist_session_id(self, &id);
    }

    fn login_id(&mut self, user_id: impl Into<String>) {
        let id = user_id.into();
        rotate_session(self);
        self.set(Authenticated { id: id.clone() });
        self.set(RateLimitIdentity(id.clone()));
        persist_session_id(self, &id);
    }

    fn logout(&mut self) {
        #[cfg(feature = "session")]
        {
            use ruvo_session::SessionExt;
            if let Some(state) = self.try_state::<PassportState>() {
                self.session().remove(&state.session_key);
            }
            // New sid so the old cookie cannot be reused as the same login.
            self.session().regenerate();
        }
        let _ = self.take::<Authenticated>();
        let _ = self.take::<RateLimitIdentity>();
    }

    fn is_authenticated(&self) -> bool {
        self.get::<Authenticated>().is_some()
    }

    fn user_id(&self) -> Option<String> {
        self.get::<Authenticated>().map(|a| a.id.clone())
    }

    fn user<U: Send + Sync + 'static>(&self) -> Option<&U> {
        self.get::<U>()
    }

    fn require_user<U: Send + Sync + 'static>(&self) -> Result<&U> {
        self.get::<U>().ok_or(Error::Unauthorized)
    }
}

fn rotate_session(req: &Request) {
    #[cfg(feature = "session")]
    {
        use ruvo_session::SessionExt;
        // Session fixation: new sid, keep flash/other data.
        req.session().regenerate();
    }
    #[cfg(not(feature = "session"))]
    {
        let _ = req;
    }
}

fn persist_session_id(req: &Request, id: &str) {
    #[cfg(feature = "session")]
    {
        use ruvo_session::SessionExt;
        if let Some(state) = req.try_state::<PassportState>() {
            let sess = req.session();
            sess.bind_user(id);
            sess.set(&state.session_key, id);
            let _ = &state.serialize;
        }
    }
    #[cfg(not(feature = "session"))]
    {
        let _ = (req, id);
    }
}

/// After setting the user on `req`, run configured `serialize_user` and store the id.
pub async fn passport_serialize(req: &Request) -> Result<()> {
    let Some(state) = req.try_state::<PassportState>() else {
        return Ok(());
    };
    let Some(ser) = &state.serialize else {
        return Ok(());
    };
    if let Some(id) = ser(req).await? {
        #[cfg(feature = "session")]
        {
            use ruvo_session::SessionExt;
            req.session().set(&state.session_key, id);
        }
        #[cfg(not(feature = "session"))]
        {
            let _ = id;
        }
    }
    Ok(())
}
