//! OAuth2 login plugin (Passport / Express-lite).

mod flow;
mod provider;
mod store;

/// Built-in IdP drivers (`github`, `google`, `apple`) — add your own beside these.
pub mod drivers;

pub use provider::{OauthProvider, ProfileKind};

/// Test helpers (PKCE / state / profile parse) — not part of the stable app API.
#[doc(hidden)]
pub mod test_support {
    pub use super::flow::{
        decode_jwt_payload, now_secs, parse_profile, pkce_challenge, random_urlsafe, sign_state,
        verify_state, FlowState,
    };
    pub use super::callback_params;
}

use crate::entity::user;
use crate::store::{find_user_by_id, issue_token_pair, AuthUser};
use flow::{
    authorize_url, cookie_decode, cookie_value, exchange_code, now_secs, pkce_challenge,
    random_urlsafe, resolve_profile, sign_state, verify_state, FlowState,
};
use provider::OauthProvider as Provider;
use ruvo_core::extend::BoxFuture;
use ruvo_core::{
    App, Error, IntoResponse, Json, Plugin, Redirect, Request, Response, Result, Router,
};
use ruvo_db::DbExt;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

const STATE_COOKIE: &str = "ruvo_oauth_state";
const STATE_TTL_SECS: u64 = 600;

/// Normalized profile from the IdP.
#[derive(Debug, Clone)]
pub struct OauthProfile {
    pub provider_user_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub raw: serde_json::Value,
}

/// Tokens from the token endpoint.
#[derive(Debug, Clone)]
pub struct OauthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    /// OpenID `id_token` when the IdP returns one (Google / Apple).
    pub id_token: Option<String>,
}

type VerifyFn =
    Arc<dyn Fn(OauthProfile, OauthTokens, Request) -> BoxFuture<Result<(AuthUser, Request)>> + Send + Sync>;
type SuccessFn = Arc<dyn Fn(AuthUser, Request) -> BoxFuture<Result<Response>> + Send + Sync>;

/// Passport-style OAuth2 plugin.
pub struct Oauth {
    providers: HashMap<String, Provider>,
    mount: String,
    public_url: String,
    state_secret: String,
    verify: Option<VerifyFn>,
    success: Option<SuccessFn>,
    http: reqwest::Client,
}

#[derive(Clone)]
struct OauthState {
    providers: Arc<HashMap<String, Provider>>,
    mount: String,
    public_url: String,
    state_secret: String,
    verify: Option<VerifyFn>,
    success: Option<SuccessFn>,
    http: reqwest::Client,
}

impl Oauth {
    pub fn new() -> Self {
        let state_secret = std::env::var("OAUTH_STATE_SECRET")
            .or_else(|_| std::env::var("JWT_SECRET"))
            .unwrap_or_default();
        let public_url = std::env::var("OAUTH_PUBLIC_URL")
            .or_else(|_| std::env::var("APP_URL"))
            .unwrap_or_else(|_| "http://127.0.0.1:3000".into());
        Self {
            providers: HashMap::new(),
            mount: "/oauth".into(),
            public_url: public_url.trim_end_matches('/').into(),
            state_secret,
            verify: None,
            success: None,
            http: reqwest::Client::new(),
        }
    }

    pub fn mount(mut self, path: impl Into<String>) -> Self {
        self.mount = path.into();
        self
    }

    pub fn public_url(mut self, url: impl Into<String>) -> Self {
        self.public_url = url.into().trim_end_matches('/').into();
        self
    }

    pub fn state_secret(mut self, secret: impl Into<String>) -> Self {
        self.state_secret = secret.into();
        self
    }

    pub fn provider(mut self, p: impl Into<OauthProvider>) -> Self {
        let p = p.into();
        self.providers.insert(p.name.clone(), p);
        self
    }

    /// `(profile, tokens, req) -> (AuthUser, req)`. Default: find-or-create in DB.
    pub fn verify<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(OauthProfile, OauthTokens, Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(AuthUser, Request)>> + Send + 'static,
    {
        self.verify = Some(Arc::new(move |p, t, r| Box::pin(f(p, t, r))));
        self
    }

    /// After verify: default issues JWT [`crate::TokenPair`] JSON.
    pub fn success<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(AuthUser, Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.success = Some(Arc::new(move |u, r| Box::pin(f(u, r))));
        self
    }
}

impl Default for Oauth {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Oauth {
    fn id(&self) -> &'static str {
        "oauth"
    }

    fn requires(&self) -> &'static [&'static str] {
        &["db"]
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("OAuth")
            .description("OAuth2 login (authorization code + PKCE)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        if self.state_secret.is_empty() {
            app.on_startup(|_s| async {
                Err(Error::Internal(
                    "OAUTH_STATE_SECRET or JWT_SECRET required for Oauth".into(),
                ))
            });
            return;
        }
        if self.providers.is_empty() {
            app.on_startup(|_s| async {
                Err(Error::Internal(
                    "Oauth: add at least one .provider(...)".into(),
                ))
            });
            return;
        }
        for p in self.providers.values() {
            if let Err(e) = p.validate() {
                let msg = e.to_string();
                app.on_startup(move |_s| {
                    let msg = msg.clone();
                    async move { Err(Error::Internal(msg)) }
                });
                return;
            }
        }

        let state = OauthState {
            providers: Arc::new(self.providers),
            mount: self.mount.clone(),
            public_url: self.public_url,
            state_secret: self.state_secret,
            verify: self.verify,
            success: self.success,
            http: self.http,
        };
        app.state(state);

        let mut r = Router::new();
        r.get("/:provider", start_handler);
        r.get("/:provider/callback", callback_handler);
        r.post("/:provider/callback", callback_handler);
        app.mount(&self.mount, r);
    }
}

fn redirect_uri(state: &OauthState, provider: &Provider) -> String {
    if let Some(uri) = &provider.redirect_uri {
        return uri.clone();
    }
    format!(
        "{}{}/{}/callback",
        state.public_url, state.mount, provider.name
    )
}

fn set_oauth_cookie(mut res: Response, value: &str) -> Response {
    let cookie = format!(
        "{STATE_COOKIE}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={STATE_TTL_SECS}",
        cookie_value(value)
    );
    res.headers_mut().append(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&cookie).unwrap_or_else(|_| http::HeaderValue::from_static("")),
    );
    res
}

fn clear_oauth_cookie(mut res: Response) -> Response {
    let cookie = format!("{STATE_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    res.headers_mut().append(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&cookie).unwrap_or_else(|_| http::HeaderValue::from_static("")),
    );
    res
}

fn read_oauth_cookie(req: &Request) -> Option<String> {
    let raw = req.header("cookie")?;
    for part in raw.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            if k.trim() == STATE_COOKIE {
                return cookie_decode(v.trim()).ok();
            }
        }
    }
    None
}

async fn start_handler(req: Request) -> Result<Response> {
    let name = req
        .param("provider")
        .ok_or_else(|| Error::BadRequest("missing provider".into()))?
        .to_string();
    let state = Arc::clone(&req.state::<OauthState>());
    let provider = state
        .providers
        .get(&name)
        .ok_or_else(|| Error::NotFound)?
        .clone();

    let verifier = random_urlsafe(32);
    let challenge = pkce_challenge(&verifier);
    let flow = FlowState {
        provider: name.clone(),
        code_verifier: verifier,
        nonce: random_urlsafe(16),
        exp: now_secs() + STATE_TTL_SECS,
    };
    let signed = sign_state(&state.state_secret, &flow)?;
    let redir = redirect_uri(&state, &provider);
    let url = authorize_url(&provider, &redir, &signed, &challenge)?;

    let res = Redirect::to(url).into_response();
    Ok(set_oauth_cookie(res, &signed))
}

async fn callback_handler(mut req: Request) -> Result<Response> {
    let name = req
        .param("provider")
        .ok_or_else(|| Error::BadRequest("missing provider".into()))?
        .to_string();

    let (code, q_state, apple_user) = callback_params(&mut req).await?;

    let cookie_state = read_oauth_cookie(&req)
        .ok_or_else(|| Error::BadRequest("missing oauth state cookie".into()))?;
    if cookie_state != q_state {
        return Err(Error::BadRequest("oauth state mismatch".into()));
    }

    let state = Arc::clone(&req.state::<OauthState>());
    let flow = verify_state(&state.state_secret, &q_state)?;
    if flow.provider != name {
        return Err(Error::BadRequest("oauth provider mismatch".into()));
    }
    let provider = state
        .providers
        .get(&name)
        .ok_or(Error::NotFound)?
        .clone();
    let redir = redirect_uri(&state, &provider);

    let tokens = exchange_code(
        &state.http,
        &provider,
        &code,
        &redir,
        &flow.code_verifier,
    )
    .await?;
    let (mut profile, _raw) = resolve_profile(&state.http, &provider, &tokens).await?;
    if profile.name.is_none() {
        if let Some(n) = apple_user {
            profile.name = Some(n);
        }
    }

    let (user, req) = if let Some(verify) = &state.verify {
        verify(profile, tokens, req).await?
    } else {
        let db = req.db().clone();
        let auth_user = store::find_or_create_user(&db, &name, &profile, &tokens).await?;
        (auth_user, req)
    };

    let res = if let Some(success) = &state.success {
        success(user, req).await?
    } else {
        default_success(user, req).await?
    };
    Ok(clear_oauth_cookie(res))
}

/// Extract `code` / `state` from query (GET) or form body (Apple `form_post`).
/// Also returns Apple first-login display name from optional `user` JSON.
pub async fn callback_params(req: &mut Request) -> Result<(String, String, Option<String>)> {
    let q_code = req.query("code").map(str::to_string);
    let q_state = req.query("state").map(str::to_string);

    if let (Some(code), Some(state)) = (q_code.clone(), q_state.clone()) {
        return Ok((code, state, None));
    }

    // form_post / urlencoded body
    let form = req.form::<HashMap<String, String>>().await.unwrap_or_default();
    let code = q_code
        .or_else(|| form.get("code").cloned())
        .ok_or_else(|| Error::BadRequest("missing code".into()))?;
    let state = q_state
        .or_else(|| form.get("state").cloned())
        .ok_or_else(|| Error::BadRequest("missing state".into()))?;
    let apple_name = form
        .get("user")
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|u| {
            let name = u.get("name")?;
            let first = name.get("firstName").and_then(|v| v.as_str()).unwrap_or("");
            let last = name.get("lastName").and_then(|v| v.as_str()).unwrap_or("");
            let full = format!("{first} {last}").trim().to_string();
            if full.is_empty() {
                None
            } else {
                Some(full)
            }
        });
    Ok((code, state, apple_name))
}

async fn default_success(user: AuthUser, req: Request) -> Result<Response> {
    let model = find_user_by_id(req.db(), user.id)
        .await?
        .ok_or(Error::Unauthorized)?;
    let pair = issue_token_pair(&req, &model).await?;
    Ok(Json(pair).into_response())
}

// silence unused import if any
#[allow(dead_code)]
fn _u(_: &user::Model) {}

#[cfg(test)]
mod callback_tests {
    use super::callback_params;
    use http::Method;
    use ruvo_core::Request;

    #[tokio::test]
    async fn params_from_query() {
        let mut req = Request::builder()
            .method(Method::GET)
            .path("/cb?code=abc&state=xyz")
            .build();
        let (code, state, name) = callback_params(&mut req).await.unwrap();
        assert_eq!(code, "abc");
        assert_eq!(state, "xyz");
        assert!(name.is_none());
    }

    #[tokio::test]
    async fn params_from_form_with_apple_user() {
        let user = r#"{"name":{"firstName":"Ada","lastName":"Lovelace"}}"#;
        let body = format!(
            "code=c1&state=s1&user={}",
            url::form_urlencoded::byte_serialize(user.as_bytes()).collect::<String>()
        );
        let mut req = Request::builder()
            .method(Method::POST)
            .path("/cb")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .build();
        let (code, state, name) = callback_params(&mut req).await.unwrap();
        assert_eq!(code, "c1");
        assert_eq!(state, "s1");
        assert_eq!(name.as_deref(), Some("Ada Lovelace"));
    }
}
