//! Build an [`App`] with sqlite + migrator + plugins.

use crate::sqlite::{apply_migrations, SqliteTestDb};
use sova_core::{App, Plugin};
use sova_db::Db;
use sea_orm_migration::MigratorTrait;
use std::future::Future;
use std::pin::Pin;

type InstallFn = Box<dyn FnOnce(&mut App) + Send>;
type DbInstallFn = Box<dyn FnOnce(&mut App, String) + Send>;
type MigrateFn = Box<dyn FnOnce(&str) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// Fluent bootstrap: migrate sqlite, install `Db` + plugins, `run_startup`.
pub struct TestApp;

impl TestApp {
    pub fn builder() -> TestAppBuilder {
        TestAppBuilder {
            migrate: None,
            install_db: None,
            plugins: Vec::new(),
            env: Vec::new(),
        }
    }
}

pub struct TestAppBuilder {
    migrate: Option<MigrateFn>,
    install_db: Option<DbInstallFn>,
    plugins: Vec<InstallFn>,
    env: Vec<(String, String)>,
}

impl TestAppBuilder {
    /// Apply `M` to a new sqlite file and install `Db` with an explicit URL
    /// (avoids `DATABASE_URL` races across parallel tests).
    pub fn migrator<M: MigratorTrait + Send + Sync + 'static>(mut self) -> Self {
        self.migrate = Some(Box::new(|url| {
            let url = url.to_string();
            Box::pin(async move {
                apply_migrations::<M>(&url).await;
            })
        }));
        self.install_db = Some(Box::new(|app, url| {
            app.install(Db::from_env().url(url).migrations::<M>());
        }));
        self
    }

    /// Extra env vars (e.g. `FORTIFY_SECRET`) set before plugin install.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Install a plugin after `Db`.
    pub fn install<P: Plugin + Send + 'static>(mut self, plugin: P) -> Self {
        self.plugins.push(Box::new(move |app| {
            app.install(plugin);
        }));
        self
    }

    /// Mutate the [`App`] after plugins (routes, extra middleware) before startup.
    pub fn configure<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut App) + Send + 'static,
    {
        self.plugins.push(Box::new(f));
        self
    }

    /// Create sqlite, migrate, install plugins, [`App::run_startup`].
    pub async fn build(self) -> (SqliteTestDb, App) {
        let db = SqliteTestDb::create();
        let url = db.url().to_string();
        if let Some(migrate) = self.migrate {
            migrate(&url).await;
        }
        for (k, v) in &self.env {
            std::env::set_var(k, v);
        }
        // Keep DATABASE_URL aligned with this test's file for any late readers.
        std::env::set_var("DATABASE_URL", &url);
        let mut app = App::new();
        if let Some(install_db) = self.install_db {
            install_db(&mut app, url);
        }
        for install in self.plugins {
            install(&mut app);
        }
        app.run_startup().await.expect("run_startup");
        (db, app)
    }
}
