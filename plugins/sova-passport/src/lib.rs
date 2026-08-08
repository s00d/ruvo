//! Passport-style authentication for Sova.
//!
//! - [`Passport`] — strategy registry, `authenticate`, session serialize/deserialize, login/logout
//! - [`Auth`] / [`AuthMw`] — extract + verify strategies (Bearer, API key, JWT)
//! - feature `jwt`: [`JwtAuth`] (users, refresh, migrate)
//! - feature `oauth`: [`Oauth`] (OAuth2 code + PKCE)

mod extract;
mod passport;

#[cfg(feature = "jwt")]
mod jwt;

#[cfg(feature = "jwt")]
mod api_token;
#[cfg(feature = "jwt")]
mod entity;
#[cfg(feature = "jwt")]
mod jwt_auth;
#[cfg(feature = "jwt")]
mod migration;
#[cfg(feature = "jwt")]
mod password;
#[cfg(feature = "jwt")]
mod store;

#[cfg(feature = "oauth")]
mod oauth;

pub use extract::{Credentials, Extract, Source};
pub use passport::{passport_serialize, Authenticated, Passport, PassportExt};

#[cfg(feature = "jwt")]
pub use jwt::{Claims, Jwt, JwtError};

#[cfg(feature = "jwt")]
pub use jwt_auth::{JwtAuth, JwtAuthExt, JwtAuthState};
#[cfg(feature = "jwt")]
pub use api_token::{
    create_api_token, list_api_tokens, revoke_api_token, token_can, user_for_api_token,
    ApiTokenInfo, ApiTokenRow, CreateApiToken, CreatedApiToken, PAT_PREFIX,
};
#[cfg(feature = "jwt")]
pub use migration::AuthMigrator;
#[cfg(feature = "jwt")]
pub use password::{hash_password, verify_password};
#[cfg(feature = "jwt")]
pub use store::{
    hash_refresh_token, hash_token, issue_token_pair, register_user, AuthUser, TokenPair,
};

#[cfg(feature = "oauth")]
pub use oauth::{
    drivers as oauth_drivers, test_support as oauth_test_support, Oauth, OauthProfile,
    OauthProvider, OauthTokens, ProfileKind,
};
#[cfg(feature = "oauth")]
pub use oauth::drivers::{Apple, Custom, Driver, Github, Google};

use extract::Extract as ExtractChain;
use sova_core::extend::{named, BoxFuture, MwEntry};
use sova_core::{with_state, App, Error, Plugin, Request, Result};
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

type VerifyFn<U> =
    Arc<dyn Fn(Credentials, &Request) -> BoxFuture<Result<Option<U>>> + Send + Sync>;

/// Builder before [`AuthBuilder::verify`] binds the user type.
pub struct AuthBuilder {
    extract: ExtractChain,
    skip: Vec<String>,
    optional: bool,
    name: String,
}

/// Strategy middleware: extract credential → verify → `req.set(U)`.
pub struct AuthMw<U> {
    extract: ExtractChain,
    verify: VerifyFn<U>,
    skip: Vec<String>,
    optional: bool,
    name: String,
    _marker: PhantomData<U>,
}

impl<U: Send + Sync + 'static> AuthMw<U> {
    pub fn skip(mut self, path: impl Into<String>) -> Self {
        self.skip.push(path.into());
        self
    }

    pub fn optional(mut self, on: bool) -> Self {
        self.optional = on;
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Middleware entry for [`Passport::strategy`] / `Router::use_middleware`.
    pub fn middleware(self) -> MwEntry {
        let name = self.name.clone();
        named(
            name,
            with_state(self, |auth, mut req, next| async move {
                if path_skipped(&req.path, &auth.skip) {
                    return next(req).await;
                }

                let Some(creds) = auth.extract.run(&req) else {
                    if auth.optional {
                        return next(req).await;
                    }
                    return Error::Unauthorized.into_response();
                };

                match (auth.verify)(creds, &req).await {
                    Ok(Some(user)) => {
                        req.set(Authenticated {
                            id: std::any::type_name::<U>().into(),
                        });
                        req.set(user);
                        next(req).await
                    }
                    Ok(None) => {
                        if auth.optional {
                            next(req).await
                        } else {
                            Error::Unauthorized.into_response()
                        }
                    }
                    Err(err) => err.into_response(),
                }
            }),
        )
    }
}

impl AuthBuilder {
    pub fn new() -> Self {
        Self {
            extract: ExtractChain::bearer(),
            skip: Vec::new(),
            optional: false,
            name: "passport-strategy".into(),
        }
    }

    pub fn extract(mut self, extract: ExtractChain) -> Self {
        self.extract = extract;
        self
    }

    pub fn skip(mut self, path: impl Into<String>) -> Self {
        self.skip.push(path.into());
        self
    }

    pub fn optional(mut self, on: bool) -> Self {
        self.optional = on;
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn verify<U, F, Fut>(self, f: F) -> AuthMw<U>
    where
        U: Send + Sync + 'static,
        F: Fn(Credentials, &Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<U>>> + Send + 'static,
    {
        self.verify_boxed(move |creds, req| Box::pin(f(creds, req)))
    }

    pub fn verify_boxed<U, F>(self, f: F) -> AuthMw<U>
    where
        U: Send + Sync + 'static,
        F: Fn(Credentials, &Request) -> BoxFuture<Result<Option<U>>> + Send + Sync + 'static,
    {
        AuthMw {
            extract: self.extract,
            verify: Arc::new(f),
            skip: self.skip,
            optional: self.optional,
            name: self.name,
            _marker: PhantomData,
        }
    }
}

impl Default for AuthBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Strategy constructors: [`Auth::bearer`], [`Auth::api_key`], [`Auth::jwt_hs256`], …
pub struct Auth;

impl Auth {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> AuthBuilder {
        AuthBuilder::new()
    }

    pub fn bearer<U, F, Fut>(f: F) -> AuthMw<U>
    where
        U: Send + Sync + 'static,
        F: Fn(String, &Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<U>>> + Send + 'static,
    {
        Auth::new()
            .name("bearer")
            .extract(ExtractChain::bearer())
            .verify(move |creds, req| f(creds.value().to_string(), req))
    }

    pub fn api_key<U, F, Fut>(header: impl Into<String>, f: F) -> AuthMw<U>
    where
        U: Send + Sync + 'static,
        F: Fn(String, &Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<U>>> + Send + 'static,
    {
        Auth::new()
            .name("api-key")
            .extract(ExtractChain::header(header))
            .verify(move |creds, req| f(creds.value().to_string(), req))
    }

    #[cfg(feature = "jwt")]
    pub fn jwt_hs256<U, C, F, Fut>(secret: impl Into<String>, f: F) -> AuthMw<U>
    where
        U: Send + Sync + 'static,
        C: serde::de::DeserializeOwned + Send + 'static,
        F: Fn(C, &Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<U>>> + Send + 'static,
    {
        let jwt = Jwt::hs256(secret);
        Auth::new()
            .name("jwt")
            .extract(ExtractChain::bearer())
            .verify_boxed(move |creds, req| match jwt.decode::<C>(creds.value()) {
                Ok(claims) => Box::pin(f(claims, req)),
                Err(_) => Box::pin(async { Ok(None) }),
            })
    }
}

/// Proper local strategy (JSON email/password) — body consumed once.
pub fn local_strategy<U, F, Fut>(f: F) -> MwEntry
where
    U: Send + Sync + 'static,
    F: Fn(String, String, Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(Option<U>, Request)>> + Send + 'static,
{
    let f = Arc::new(f);
    named(
        "local",
        with_state(f, |f, mut req, next| async move {
            #[derive(serde::Deserialize)]
            struct Body {
                email: String,
                password: String,
            }
            let body: Body = match req.json().await {
                Ok(b) => b,
                Err(e) => return e.into_response(),
            };
            match f(body.email, body.password, req).await {
                Ok((Some(user), mut req)) => {
                    req.set(Authenticated {
                        id: std::any::type_name::<U>().into(),
                    });
                    req.set(user);
                    next(req).await
                }
                Ok((None, _)) => Error::Unauthorized.into_response(),
                Err(e) => e.into_response(),
            }
        }),
    )
}

impl<U: Send + Sync + 'static> Plugin for AuthMw<U> {
    fn id(&self) -> &'static str {
        "passport-auth-mw"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Passport strategy")
            .description("Extract + verify authentication strategy")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.use_middleware(self.middleware());
    }
}

fn path_skipped(path: &str, skips: &[String]) -> bool {
    skips.iter().any(|p| {
        if let Some(prefix) = p.strip_suffix('*') {
            path.starts_with(prefix)
        } else {
            path == p
        }
    })
}

/// Prefer [`PassportExt`] — kept as a deprecated alias name for docs only.
#[doc(inline)]
pub use PassportExt as AuthExt;
