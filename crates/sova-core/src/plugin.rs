//! Plugin extension trait and SDK metadata.
//!
//! # Writing a plugin
//!
//! A plugin is any type that implements [`Plugin`]. On install it typically:
//!
//! 1. Registers middleware via [`crate::App::use_middleware`] / [`crate::extend::with_leaked`]
//! 2. Inserts shared state with [`crate::App::state`]
//! 3. Adds routes (`get` / `post` / …)
//! 4. Optionally registers lifecycle hooks, CLI commands, or checks
//!
//! ## Identity and dependencies
//!
//! Override [`Plugin::id`] with a short stable string (`"cookies"`, `"session"`).
//! Use [`Plugin::requires`] so dependents fail at [`crate::App::build`] if a
//! dependency was not installed first. Prefer short ids over `type_name`.
//!
//! ## SDK versioning
//!
//! [`PLUGIN_SDK_VERSION`] is the plugin-author surface version (independent of
//! the crate semver). Declare the version your plugin was built against via
//! [`PluginMeta::sdk`] (default = current). Compatibility on install:
//!
//! - **different major** → hard error at build
//! - **plugin newer** than core (same major) → hard error
//! - **core newer** than plugin (same major) → `tracing` warning
//!
//! Bump `PLUGIN_SDK_VERSION` major only when the author-facing API breaks.
//!
//! ## Metadata
//!
//! [`Plugin::meta`] returns human-readable info for CLI (`plugins`) and docs.
//!
//! # Examples
//!
//! ```rust,ignore
//! use sova_core::extend::with_leaked;
//! use sova_core::{App, Plugin, PluginMeta, Request, Response};
//!
//! struct HelloHeader;
//!
//! impl Plugin for HelloHeader {
//!     fn id(&self) -> &'static str {
//!         "hello-header"
//!     }
//!
//!     fn meta(&self) -> PluginMeta {
//!         PluginMeta::new("Hello Header")
//!             .description("Adds an X-Hello response header")
//!             .version(env!("CARGO_PKG_VERSION"))
//!     }
//!
//!     fn install(self, app: &mut App) {
//!         app.use_middleware(with_leaked((), |_s, req, next| async move {
//!             let mut res = next(req).await;
//!             res = res.header("x-hello", "sova");
//!             res
//!         }));
//!     }
//! }
//!
//! let mut app = App::new();
//! app.install(HelloHeader);
//! ```
//!
//! Closure plugins work without a named type:
//!
//! ```rust,ignore
//! app.install(|app: &mut App| {
//!     app.get("/healthz", || async { Response::text("ok") });
//! });
//! ```
//!
//! Scaffold a new crate with `cargo sovax generate plugin <name>`.

use crate::app::App;
use std::cmp::Ordering;
use std::fmt;

/// Current Plugin SDK version (author-facing surface, not crate semver).
pub const PLUGIN_SDK_VERSION: PluginSdkVersion = PluginSdkVersion::new(1, 0, 0);

/// Semantic version of the Plugin SDK (`major.minor.patch`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginSdkVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PluginSdkVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse `"1.2.3"` / `"1.2"` / `"1"`. Invalid input → `None`.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.trim().split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().unwrap_or("0").parse().ok()?;
        let patch = parts.next().unwrap_or("0").parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self::new(major, minor, patch))
    }
}

impl fmt::Display for PluginSdkVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Result of comparing a plugin's declared SDK against core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkCompat {
    /// Exact match (or plugin == core).
    Ok,
    /// Same major; core is newer — plugin may miss new APIs later; warn only.
    Warn { core: PluginSdkVersion, plugin: PluginSdkVersion },
    /// Hard failure: major mismatch or plugin requires a newer core.
    Error(String),
}

/// Compare `plugin` SDK declaration against `core` ([`PLUGIN_SDK_VERSION`]).
pub fn check_plugin_sdk(plugin: PluginSdkVersion, core: PluginSdkVersion) -> SdkCompat {
    if plugin.major != core.major {
        return SdkCompat::Error(format!(
            "plugin SDK {plugin} is incompatible with core SDK {core} (major version mismatch)"
        ));
    }
    match plugin.cmp(&core) {
        Ordering::Greater => SdkCompat::Error(format!(
            "plugin SDK {plugin} requires core SDK >= {plugin} (running {core})"
        )),
        Ordering::Less => SdkCompat::Warn { core, plugin },
        Ordering::Equal => SdkCompat::Ok,
    }
}

/// Human-readable plugin metadata (CLI, docs, introspection).
#[derive(Debug, Clone)]
pub struct PluginMeta {
    /// Display name (defaults to plugin id).
    pub name: &'static str,
    /// One-line description of what the plugin does.
    pub description: &'static str,
    /// Plugin crate / package version (not SDK), e.g. `env!("CARGO_PKG_VERSION")`.
    pub version: &'static str,
    /// Optional author / maintainer.
    pub author: &'static str,
    /// Plugin SDK version this plugin was written against.
    pub sdk: PluginSdkVersion,
}

impl PluginMeta {
    /// Start a builder with a display `name`; SDK defaults to [`PLUGIN_SDK_VERSION`].
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            description: "",
            version: "",
            author: "",
            sdk: PLUGIN_SDK_VERSION,
        }
    }

    /// Minimal meta for plugins that only set [`Plugin::id`].
    pub fn for_id(id: &'static str) -> Self {
        Self::new(id)
    }

    pub fn description(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }

    pub fn version(mut self, version: &'static str) -> Self {
        self.version = version;
        self
    }

    pub fn author(mut self, author: &'static str) -> Self {
        self.author = author;
        self
    }

    pub fn sdk(mut self, sdk: PluginSdkVersion) -> Self {
        self.sdk = sdk;
        self
    }
}

/// Snapshot of an installed plugin (for CLI / introspection).
#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    pub id: &'static str,
    pub meta: PluginMeta,
}

/// Single extension trait for the framework.
///
/// Prefer `app.install(|app| { ... })` or `app.install(Cors::new())` —
/// application code rarely needs to name this trait; plugin authors implement it.
pub trait Plugin {
    /// Stable plugin identifier used for dependency checks and [`crate::App::has_plugin`].
    ///
    /// Prefer a short constant (`"session"`) over the default [`std::any::type_name`].
    fn id(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Required plugin ids that must be installed beforehand.
    ///
    /// Missing deps are collected at install and reported on [`crate::App::build`].
    fn requires(&self) -> &'static [&'static str] {
        &[]
    }

    /// Display name, description, and declared [`PluginMeta::sdk`] version.
    fn meta(&self) -> PluginMeta {
        PluginMeta::for_id(self.id())
    }

    /// Register middleware, state, routes, and hooks on `app`.
    fn install(self, app: &mut App);
}

impl<F> Plugin for F
where
    F: FnOnce(&mut App),
{
    fn install(self, app: &mut App) {
        self(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sdk_version() {
        assert_eq!(
            PluginSdkVersion::parse("1.2.3"),
            Some(PluginSdkVersion::new(1, 2, 3))
        );
        assert_eq!(
            PluginSdkVersion::parse("2"),
            Some(PluginSdkVersion::new(2, 0, 0))
        );
        assert!(PluginSdkVersion::parse("x").is_none());
    }

    #[test]
    fn sdk_compat_rules() {
        let core = PluginSdkVersion::new(1, 1, 0);
        assert_eq!(
            check_plugin_sdk(PluginSdkVersion::new(1, 1, 0), core),
            SdkCompat::Ok
        );
        assert!(matches!(
            check_plugin_sdk(PluginSdkVersion::new(1, 0, 0), core),
            SdkCompat::Warn { .. }
        ));
        assert!(matches!(
            check_plugin_sdk(PluginSdkVersion::new(1, 2, 0), core),
            SdkCompat::Error(_)
        ));
        assert!(matches!(
            check_plugin_sdk(PluginSdkVersion::new(0, 9, 0), core),
            SdkCompat::Error(_)
        ));
    }
}
