//! Facade [`App`] / [`BoundApp`] so `listen` / `run` / `serve` return [`crate::Result`].

use crate::{AppError, Result};
use sova_core::extend::Bind;
use sova_core::Http;
use std::future::Future;
use std::ops::{Deref, DerefMut};

/// Application builder — thin wrapper over [`sova_core::App`].
pub struct App(sova_core::App);

impl App {
    pub fn new() -> Self {
        Self(sova_core::App::new())
    }

    pub fn bind(self, target: impl Into<Bind>) -> BoundApp {
        BoundApp(self.0.bind(target))
    }

    /// Bind `0.0.0.0:port` and [`Self::run`].
    pub async fn listen(self, port: u16) -> Result<()> {
        self.0.listen(port).await.map_err(AppError::from)
    }

    /// CLI helpers or serve on `HOST`/`PORT` (default port 3000).
    pub async fn run(self) -> Result<()> {
        self.0.run().await.map_err(AppError::from)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for App {
    type Target = sova_core::App;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<sova_core::App> for App {
    fn from(app: sova_core::App) -> Self {
        Self(app)
    }
}

impl From<App> for sova_core::App {
    fn from(app: App) -> Self {
        app.0
    }
}

/// Bound app — `serve` / `run` return facade [`Result`].
pub struct BoundApp(sova_core::BoundApp);

impl BoundApp {
    pub fn http(self, http: Http) -> Self {
        Self(self.0.http(http))
    }

    pub fn reuseport(self, enabled: bool) -> Self {
        Self(self.0.reuseport(enabled))
    }

    pub fn shutdown<F>(self, f: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Self(self.0.shutdown(f))
    }

    #[cfg(feature = "tls")]
    pub fn tls(self, config: sova_core::Tls) -> Result<Self> {
        Ok(Self(self.0.tls(config).map_err(AppError::from)?))
    }

    pub async fn serve(self) -> Result<()> {
        self.0.serve().await.map_err(AppError::from)
    }

    pub async fn run(self) -> Result<()> {
        self.0.run().await.map_err(AppError::from)
    }
}
