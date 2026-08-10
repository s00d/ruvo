//! Two-level i18n for Sova (root + page scopes).

mod ext;
mod mount;
mod plural;
mod resolve;
mod routes;
mod store;
mod template;

pub use ext::{interpolate, I18nExt, I18nRouteExt, I18nScope, I18nState, LocaleCode};
pub use mount::{localize_path, localized_url, mount_localized, strip_locale_prefix, PrefixMode};
pub use plural::{default_plural, PluralFn};
pub use resolve::{negotiate_accept_language, resolve_server_locale, LocaleSource, ResolveOptions};
pub use store::{get_by_path, load_store, Locale, Scope, Store, ROOT_SCOPE};
pub use template::template_fn;

use arc_swap::ArcSwap;
use ext::MissingHandler;
use sova_core::extend::named;
use sova_core::{with_state, App, Error, Plugin};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Marker: `GET /_i18n/all.json` is allowed.
#[derive(Clone, Copy)]
pub struct AllJsonEnabled(pub bool);

/// I18n plugin.
pub struct I18n {
    dir: PathBuf,
    locales: Vec<Locale>,
    default: String,
    fallback: String,
    path_prefix: bool,
    cookie_name: Option<String>,
    set_locale_cookie: bool,
    enable_all_json: bool,
    plural_fn: Option<PluralFn>,
    missing_handler: Option<MissingHandler>,
    #[cfg(feature = "watch")]
    watch: bool,
}

impl I18n {
    pub fn new(dir: impl Into<PathBuf>, locales: Vec<Locale>) -> Self {
        let default = locales
            .first()
            .map(|l| l.code.clone())
            .unwrap_or_else(|| "en".into());
        Self {
            dir: dir.into(),
            locales,
            fallback: default.clone(),
            default,
            path_prefix: true,
            cookie_name: None,
            set_locale_cookie: false,
            enable_all_json: false,
            plural_fn: None,
            missing_handler: None,
            #[cfg(feature = "watch")]
            watch: false,
        }
    }

    pub fn default_locale(mut self, code: impl Into<String>) -> Self {
        self.default = code.into();
        self
    }

    pub fn fallback(mut self, code: impl Into<String>) -> Self {
        self.fallback = code.into();
        self
    }

    pub fn path_prefix(mut self, enabled: bool) -> Self {
        self.path_prefix = enabled;
        self
    }

    /// Enable cookie locale resolution (requires feature `cookie` + `CookieLayer`).
    #[cfg(feature = "cookie")]
    pub fn cookie(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = Some(name.into());
        self
    }

    #[cfg(feature = "cookie")]
    pub fn set_locale_cookie(mut self, yes: bool) -> Self {
        self.set_locale_cookie = yes;
        self
    }

    pub fn enable_all_json(mut self, yes: bool) -> Self {
        self.enable_all_json = yes;
        self
    }

    pub fn plural_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&str, i64, &str, &[&str]) -> String + Send + Sync + 'static,
    {
        self.plural_fn = Some(Arc::new(f));
        self
    }

    pub fn missing_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(&str, &str, &str) + Send + Sync + 'static,
    {
        self.missing_handler = Some(Arc::new(f));
        self
    }

    #[cfg(feature = "watch")]
    pub fn watch(mut self, yes: bool) -> Self {
        self.watch = yes;
        self
    }
}

impl Plugin for I18n {
    fn id(&self) -> &'static str {
        "i18n"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("I18n")
            .description("Locales, catalogs, optional path prefix and cookie")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn requires(&self) -> &'static [&'static str] {
        // Feature `cookie` always needs Cookies (toml may set cookie_name at install).
        #[cfg(feature = "cookie")]
        {
            let _ = &self.cookie_name;
            return &["cookies"];
        }
        #[cfg(not(feature = "cookie"))]
        {
            &[]
        }
    }

    fn install(mut self, app: &mut App) {
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("i18n") {
                if let Some(c) = section.get("default").and_then(|v| v.as_str()) {
                    self.default = c.to_string();
                }
                #[cfg(feature = "cookie")]
                if self.cookie_name.is_none() {
                    if let Some(n) = section.get("cookie").and_then(|v| v.as_str()) {
                        self.cookie_name = Some(n.to_string());
                    }
                }
                #[cfg(feature = "watch")]
                if let Some(v) = section.get("watch").and_then(|v| v.as_bool()) {
                    self.watch = v;
                }
            }
        }
        let store = match load_store(&self.dir, &self.locales) {
            Ok(s) => s,
            Err(err) => {
                let msg = err.to_string();
                app.on_startup(move |_| {
                    let msg = msg.clone();
                    async move { Err(Error::Internal(format!("i18n: {msg}"))) }
                });
                return;
            }
        };

        let swap = Arc::new(ArcSwap::from_pointee(store));
        let state = I18nState {
            store: Arc::clone(&swap),
            fallback: self.fallback.clone().into_boxed_str(),
            default: self.default.clone().into_boxed_str(),
            path_prefix: self.path_prefix,
            plural_fn: self.plural_fn.clone(),
            missing_handler: self.missing_handler.clone(),
            missing_keys: Arc::new(Mutex::new(HashSet::new())),
        };
        app.state(state);
        app.state(AllJsonEnabled(self.enable_all_json));

        #[cfg(feature = "templates")]
        {
            use crate::template::template_fn;
            sova_templates::register_per_request(app, "t", template_fn);
        }

        let locales_dir = self.dir.clone();
        let locales_meta = self.locales.clone();
        app.register_audit("i18n", move |_state| {
            let dir = locales_dir.clone();
            let locales = locales_meta.clone();
            async move {
                load_store(&dir, &locales)?;
                Ok(())
            }
        });

        let codes: Vec<String> = self.locales.iter().map(|l| l.code.clone()).collect();
        let mut resolve_opts = ResolveOptions::new(codes, self.default.clone());
        resolve_opts.path_prefix = self.path_prefix;
        resolve_opts.cookie_name = self.cookie_name.clone();

        let set_cookie = self.set_locale_cookie;
        let cookie_name = self.cookie_name.clone();
        let opts = resolve_opts;

        app.use_middleware(named(
            "i18n",
            with_state(
                (opts, set_cookie, cookie_name),
                |cfg, mut req, next| async move {
                    let (opts, set_cookie, cookie_name) = &*cfg;
                    let resolved = resolve_server_locale(&req, opts);
                    req.set(LocaleCode(resolved.code.clone().into_boxed_str()));
                    let mut res = next(req).await;
                    res = res
                        .header("content-language", &resolved.code)
                        .header("vary", "accept-language");
                    #[cfg(feature = "cookie")]
                    if *set_cookie {
                        if let Some(name) = cookie_name {
                            use sova_cookies::{CookieBuilder, ResponseCookieExt};
                            res = res.cookie(
                                CookieBuilder::build((name.clone(), resolved.code.clone()))
                                    .path("/")
                                    .build(),
                            );
                        }
                    }
                    #[cfg(not(feature = "cookie"))]
                    let _ = (set_cookie, cookie_name);
                    res
                },
            ),
        ));

        // Exact paths before param routes.
        app.get("/_i18n/locales.json", routes::locales_json);
        app.get("/_i18n/all.json", routes::all_json);
        app.get("/_i18n/_missing.json", routes::missing_json);
        // `/_i18n/en.json` and `/_i18n/en/blog.json` (`.json` stripped in handler).
        app.get("/_i18n/:locale", routes::locale_or_scope_json);
        app.get("/_i18n/:locale/:scope", routes::locale_or_scope_json);

        #[cfg(feature = "watch")]
        if self.watch {
            spawn_watch(self.dir.clone(), self.locales.clone(), Arc::clone(&swap));
        }
    }
}

#[cfg(feature = "watch")]
fn spawn_watch(dir: PathBuf, locales: Vec<Locale>, swap: Arc<ArcSwap<Store>>) {
    use notify::{RecommendedWatcher, RecursiveMode, Watcher};
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = match RecommendedWatcher::new(tx, notify::Config::default()) {
            Ok(w) => w,
            Err(err) => {
                tracing::error!("i18n watch: {err}");
                return;
            }
        };
        if let Err(err) = watcher.watch(&dir, RecursiveMode::Recursive) {
            tracing::error!("i18n watch: {err}");
            return;
        }
        for _ in rx {
            match load_store(&dir, &locales) {
                Ok(store) => {
                    tracing::info!("i18n: reloaded translations");
                    swap.store(Arc::new(store));
                }
                Err(err) => tracing::error!("i18n reload failed: {err}"),
            }
        }
    });
}
