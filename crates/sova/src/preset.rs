//! Opinionated app presets (`web` / `api`).

use crate::{App, Result};
use std::ops::{Deref, DerefMut};
#[cfg(feature = "web")]
use std::path::PathBuf;

/// Web stack: cors, session, csrf, static, templates, meta, shield.
#[cfg(feature = "web")]
pub struct WebApp {
    inner: App,
    site: Option<String>,
    public_url: Option<String>,
    views: PathBuf,
    assets: PathBuf,
    assets_mount: String,
    installed: bool,
}

#[cfg(feature = "web")]
impl WebApp {
    pub(crate) fn new() -> Self {
        #[cfg(feature = "env")]
        let _ = sova_env::load();

        let mut inner = App::new();
        let _ = inner.configure();

        Self {
            inner,
            site: None,
            public_url: None,
            views: PathBuf::from("views"),
            assets: PathBuf::from("public"),
            assets_mount: "/assets".into(),
            installed: false,
        }
    }

    pub fn site(mut self, name: impl Into<String>) -> Self {
        self.site = Some(name.into());
        self
    }

    pub fn public_url(mut self, url: impl Into<String>) -> Self {
        self.public_url = Some(url.into());
        self
    }

    pub fn views(mut self, dir: impl Into<PathBuf>) -> Self {
        self.views = dir.into();
        self
    }

    pub fn assets(mut self, dir: impl Into<PathBuf>) -> Self {
        self.assets = dir.into();
        self
    }

    pub fn assets_mount(mut self, mount: impl Into<String>) -> Self {
        self.assets_mount = mount.into();
        self
    }

    fn ensure_installed(&mut self) {
        if self.installed {
            return;
        }
        self.installed = true;

        use crate::{
            memory_sessions, Cors, Csrf, Meta, Robots, Shield, Sitemap, Static, Templates,
        };

        self.inner.use_middleware(crate::request_id());
        self.inner.use_middleware(crate::logger());
        self.inner
            .error_handler(|err| async move { sova_core::error_response_for_accept(None, err) });
        self.inner.install(Cors::new());
        self.inner.install(Shield::new());
        self.inner.install(memory_sessions());
        self.inner.install(Csrf::new());

        if self.assets.is_dir() {
            self.inner
                .install(Static::new(self.assets_mount.clone(), self.assets.clone()));
        }

        if self.views.is_dir() {
            self.inner.install(Templates::minijinja(&self.views));
        }

        let mut meta = Meta::new();
        if let Some(ref site) = self.site {
            meta = meta
                .site_name(site.clone())
                .title_template(format!("{{}} — {site}"));
        }
        if let Some(ref url) = self.public_url {
            meta = meta.public_url(url.clone());
        }
        self.inner.install(meta);
        self.inner.install(Sitemap::new());
        self.inner.install(Robots::new());
        self.inner.with_probes();
    }

    /// Bind `0.0.0.0:port` and serve.
    pub async fn listen(mut self, port: u16) -> Result<()> {
        self.ensure_installed();
        self.inner.listen(port).await
    }

    /// CLI helpers or `HOST`/`PORT` (default 3000).
    pub async fn run(mut self) -> Result<()> {
        self.ensure_installed();
        self.inner.run().await
    }

    /// Finish preset install and return the underlying [`App`].
    pub fn into_app(mut self) -> App {
        self.ensure_installed();
        self.inner
    }
}

#[cfg(feature = "web")]
impl Deref for WebApp {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "web")]
impl DerefMut for WebApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ensure_installed();
        &mut self.inner
    }
}

/// API stack: cors, session, OpenAPI docs.
#[cfg(feature = "api")]
pub struct ApiApp {
    inner: App,
    title: String,
    version: String,
    docs_mount: String,
    installed: bool,
}

#[cfg(feature = "api")]
impl ApiApp {
    pub(crate) fn new() -> Self {
        #[cfg(feature = "env")]
        let _ = sova_env::load();

        let mut inner = App::new();
        let _ = inner.configure();

        Self {
            inner,
            title: "API".into(),
            version: "1.0".into(),
            docs_mount: "/docs".into(),
            installed: false,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn docs_mount(mut self, mount: impl Into<String>) -> Self {
        self.docs_mount = mount.into();
        self
    }

    fn ensure_installed(&mut self) {
        if self.installed {
            return;
        }
        self.installed = true;

        use crate::{memory_sessions, Cors, OpenApi};

        self.inner.use_middleware(crate::request_id());
        self.inner.use_middleware(crate::logger());
        self.inner
            .error_handler(|err| async move { sova_core::error_to_problem(err) });
        self.inner.install(Cors::new());
        self.inner.install(memory_sessions());
        self.inner.install(
            OpenApi::new(self.title.clone(), self.version.clone()).mount(self.docs_mount.clone()),
        );
        #[cfg(feature = "vld")]
        self.inner.install(crate::Vld);
        self.inner.with_probes();
    }

    pub async fn listen(mut self, port: u16) -> Result<()> {
        self.ensure_installed();
        self.inner.listen(port).await
    }

    pub async fn run(mut self) -> Result<()> {
        self.ensure_installed();
        self.inner.run().await
    }

    pub fn into_app(mut self) -> App {
        self.ensure_installed();
        self.inner
    }
}

#[cfg(feature = "api")]
impl Deref for ApiApp {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "api")]
impl DerefMut for ApiApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ensure_installed();
        &mut self.inner
    }
}

impl App {
    /// Opinionated web preset (feature `web`).
    #[cfg(feature = "web")]
    pub fn web() -> WebApp {
        WebApp::new()
    }

    /// Opinionated API preset (feature `api`).
    #[cfg(feature = "api")]
    pub fn api() -> ApiApp {
        ApiApp::new()
    }
}
