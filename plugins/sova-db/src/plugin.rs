use crate::handle::DbPool;
use crate::migrate_cli::run_migrate;
use crate::tx::inject_conn;
use sova_core::extend::StateMap;
use sova_core::{App, Error, Plugin};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type MigrateFn = Arc<
    dyn Fn(
            DatabaseConnection,
            Vec<String>,
        ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>
        + Send
        + Sync,
>;

type SeedFn = Arc<
    dyn Fn(Arc<StateMap>) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>
        + Send
        + Sync,
>;

/// SeaORM pool plugin (backend selected by URL + Cargo features).
pub struct Db {
    url: String,
    /// When true, [`Self::url`] wins over `DATABASE_URL` / toml at install time.
    url_pinned: bool,
    migrate: Option<MigrateFn>,
    seed: Option<SeedFn>,
}

impl Db {
    pub fn from_env() -> Self {
        let url = std::env::var("DATABASE_URL").unwrap_or_default();
        Self {
            url,
            url_pinned: false,
            migrate: None,
            seed: None,
        }
    }

    /// Pin the connection URL (takes precedence over `DATABASE_URL` and `[db] url`).
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self.url_pinned = true;
        self
    }

    /// Register `myapp migrate [up|down|status] [N]` CLI hooks.
    pub fn migrations<M: MigratorTrait + 'static>(mut self) -> Self {
        self.migrate = Some(Arc::new(move |conn, args| {
            Box::pin(async move { run_migrate::<M>(conn, &args).await })
        }));
        self
    }

    /// Register `myapp seed` CLI (runs after DB startup; not on every server start).
    ///
    /// Accepts `Result<(), E>` where `E: Into<Error>` so facade `AppError` works with `?`.
    pub fn seed<F, Fut, E>(mut self, f: F) -> Self
    where
        F: Fn(Arc<StateMap>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), E>> + Send + 'static,
        E: Into<Error> + Send + 'static,
    {
        self.seed = Some(Arc::new(move |state| {
            let fut = f(state);
            Box::pin(async move { fut.await.map_err(Into::into) })
        }));
        self
    }
}

impl Plugin for Db {
    fn id(&self) -> &'static str {
        "db"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Database")
            .description("SeaORM pool, migrate CLI, optional seed CLI")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        // Pinned `.url()` wins; else `DATABASE_URL`, then `[db] url` in toml.
        if !self.url_pinned {
            if let Ok(u) = std::env::var("DATABASE_URL") {
                if !u.is_empty() {
                    self.url = u;
                }
            }
            if self.url.is_empty() {
                if let Some(u) = app
                    .config_doc()
                    .and_then(|d| d.section("db"))
                    .and_then(|s| s.get("url").and_then(|v| v.as_str()).map(str::to_string))
                {
                    self.url = u;
                }
            }
        }

        if self.url.is_empty() {
            app.on_startup(|_state| async {
                Err(Error::Internal(
                    "database url is empty; set DATABASE_URL or [db] url in sova.toml".into(),
                ))
            });
            return;
        }

        let pool = DbPool::new();
        app.state(pool.clone());

        let url = self.url.clone();
        let pool_start = pool.clone();
        app.on_startup(move |_state| {
            let url = url.clone();
            let pool = pool_start.clone();
            async move {
                let conn = Database::connect(&url)
                    .await
                    .map_err(|e| Error::Internal(format!("db connect: {e}")))?;
                conn.ping()
                    .await
                    .map_err(|e| Error::Internal(format!("db ping: {e}")))?;
                pool.set(conn).await;
                Ok(())
            }
        });

        let pool_stop = pool.clone();
        app.on_shutdown(move || {
            let pool = pool_stop.clone();
            async move {
                pool.clear().await;
            }
        });

        app.use_middleware(inject_conn(pool.clone()));

        let pool_check = pool.clone();
        app.register_check("db", move |_state| {
            let pool = pool_check.clone();
            async move {
                let conn = pool.get().await.map_err(Error::from)?;
                conn.ping()
                    .await
                    .map_err(|e| Error::Internal(format!("db ping: {e}")))?;
                Ok(())
            }
        });

        if let Some(migrate) = self.migrate {
            let pool_cli = pool.clone();
            app.register_cli("migrate", move |_state, args| {
                let pool = pool_cli.clone();
                let migrate = Arc::clone(&migrate);
                async move {
                    let conn = pool.get().await.map_err(Error::from)?;
                    migrate(conn, args).await
                }
            });
        }

        if let Some(seed) = self.seed {
            app.register_cli("seed", move |state, _args| {
                let seed = Arc::clone(&seed);
                async move { seed(state).await }
            });
        }
    }
}
