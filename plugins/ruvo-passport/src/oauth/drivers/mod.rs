//! Built-in OAuth IdP drivers — one struct per file, shared [`Driver`] methods.
//!
//! ```ignore
//! use ruvo::oauth_drivers::{Apple, Driver, Github, Google, Custom};
//!
//! Oauth::new()
//!     .provider(Github::new().from_env())
//!     .provider(Google::new().from_env())
//!     .provider(Apple::new().from_env())
//!     .provider(
//!         Custom::new(
//!             "discord",
//!             "https://discord.com/api/oauth2/authorize",
//!             "https://discord.com/api/oauth2/token",
//!             "https://discord.com/api/users/@me",
//!         )
//!         .scopes(["identify", "email"])
//!         .from_env(),
//!     );
//! ```

mod apple;
mod custom;
mod github;
mod google;

pub use apple::Apple;
pub use custom::Custom;
pub use github::Github;
pub use google::Google;

use super::provider::OauthProvider;

/// Shared builder surface for every OAuth driver.
pub trait Driver: Sized {
    fn into_provider(self) -> OauthProvider;
    fn from_provider(provider: OauthProvider) -> Self;

    fn client_id(self, id: impl Into<String>) -> Self {
        Self::from_provider(self.into_provider().client_id(id))
    }

    fn client_secret(self, secret: impl Into<String>) -> Self {
        Self::from_provider(self.into_provider().client_secret(secret))
    }

    fn scopes(self, scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::from_provider(self.into_provider().scopes(scopes))
    }

    fn redirect_uri(self, uri: impl Into<String>) -> Self {
        Self::from_provider(self.into_provider().redirect_uri(uri))
    }

    fn userinfo_url(self, url: impl Into<String>) -> Self {
        Self::from_provider(self.into_provider().userinfo_url(url))
    }

    fn auth_param(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::from_provider(self.into_provider().auth_param(key, value))
    }

    fn auth_params(
        self,
        params: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self::from_provider(self.into_provider().auth_params(params))
    }

    /// Load `{NAME}_CLIENT_ID` / `_CLIENT_SECRET` / `_REDIRECT_URI` (and driver-specific env).
    #[allow(clippy::wrong_self_convention)]
    fn from_env(self) -> Self {
        Self::from_provider(self.into_provider().from_env())
    }

    fn build(self) -> OauthProvider {
        self.into_provider()
    }
}

macro_rules! impl_driver_from {
    ($t:ty) => {
        impl From<$t> for OauthProvider {
            fn from(d: $t) -> Self {
                d.into_provider()
            }
        }
    };
}

pub(crate) use impl_driver_from;

pub(crate) use apple::mint_client_secret as mint_apple_secret;
