use crate::handle::DbPool;
use crate::migrate_cli::run_migrate;
use crate::tx::inject_conn;
use sova_core::extend::StateMap;
use sova_core::{App, Error, Plugin};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::future::Future;
use std::path::Path;
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
    /// Emit sqlx query tracing events (for DevTools / RUST_LOG=sqlx=debug).
    sqlx_logging: bool,
    migrate: Option<MigrateFn>,
    seed: Option<SeedFn>,
    migrate_on_startup: bool,
    seed_on_startup: bool,
}

impl Db {
    pub fn from_env() -> Self {
        let url = std::env::var("DATABASE_URL").unwrap_or_default();
        Self {
            url,
            url_pinned: false,
            sqlx_logging: false,
            migrate: None,
            seed: None,
            migrate_on_startup: false,
            seed_on_startup: false,
        }
    }

    /// Pin the connection URL (takes precedence over `DATABASE_URL` and `[db] url`).
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self.url_pinned = true;
        self
    }

    /// Enable sqlx statement logging via tracing (DevTools DB tab / `RUST_LOG=sqlx=debug`).
    pub fn sqlx_logging(mut self, on: bool) -> Self {
        self.sqlx_logging = on;
        self
    }

    /// Apply pending migrations after connect (also still available as `migrate` CLI).
    pub fn migrate_on_startup(mut self) -> Self {
        self.migrate_on_startup = true;
        self
    }

    /// Run [`Self::seed`] after connect / migrate (also still available as `seed` CLI).
    pub fn seed_on_startup(mut self) -> Self {
        self.seed_on_startup = true;
        self
    }

    /// Register `myapp migrate [up|down|status] [N]` CLI hooks.
    pub fn migrations<M: MigratorTrait + 'static>(mut self) -> Self {
        self.migrate = Some(Arc::new(move |conn, args| {
            Box::pin(async move { run_migrate::<M>(conn, &args).await })
        }));
        self
    }

    /// Register `myapp seed` CLI (and optionally [`Self::seed_on_startup`]).
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
                if let Some(doc) = app.config_doc() {
                    if let Some(u) = doc
                        .section("db")
                        .and_then(|s| s.get("url").and_then(|v| v.as_str()).map(str::to_string))
                    {
                        self.url = resolve_sqlite_url(&u, doc.source_dir.as_deref());
                    }
                }
            }
        }

        // Toml can enable auto migrate/seed without code changes.
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("db") {
                if section
                    .get("migrate_on_startup")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    self.migrate_on_startup = true;
                }
                if section
                    .get("seed_on_startup")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    self.seed_on_startup = true;
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
        let sqlx_logging = self.sqlx_logging;
        let migrate_boot = self
            .migrate_on_startup
            .then(|| self.migrate.clone())
            .flatten();
        app.on_startup(move |_state| {
            let url = url.clone();
            let pool = pool_start.clone();
            let migrate_boot = migrate_boot.clone();
            async move {
                let mut opt = ConnectOptions::new(url);
                opt.sqlx_logging(sqlx_logging);
                let conn = Database::connect(opt)
                    .await
                    .map_err(|e| Error::Internal(format!("db connect: {e}")))?;
                conn.ping()
                    .await
                    .map_err(|e| Error::Internal(format!("db ping: {e}")))?;
                if let Some(migrate) = migrate_boot {
                    // empty args → migrate up (all pending)
                    migrate(conn.clone(), Vec::new()).await?;
                }
                pool.set(conn);
                Ok(())
            }
        });

        let pool_stop = pool.clone();
        app.on_shutdown(move || {
            let pool = pool_stop.clone();
            async move {
                pool.clear();
            }
        });

        app.use_middleware(inject_conn(pool.clone()));

        let pool_check = pool.clone();
        app.register_check("db", move |_state| {
            let pool = pool_check.clone();
            async move {
                let conn = pool.get().map_err(Error::from)?;
                conn.ping()
                    .await
                    .map_err(|e| Error::Internal(format!("db ping: {e}")))?;
                Ok(())
            }
        });

        if let Some(migrate) = self.migrate.clone() {
            let pool_cli = pool.clone();
            app.register_cli("migrate", move |_state, args| {
                let pool = pool_cli.clone();
                let migrate = Arc::clone(&migrate);
                async move {
                    let conn = pool.get().map_err(Error::from)?;
                    migrate(conn, args).await
                }
            });
        }

        if let Some(seed) = self.seed.clone() {
            let seed_cli = Arc::clone(&seed);
            app.register_cli("seed", move |state, _args| {
                let seed = Arc::clone(&seed_cli);
                async move { seed(state).await }
            });
            if self.seed_on_startup {
                app.on_startup(move |state| {
                    let seed = Arc::clone(&seed);
                    async move { seed(state).await }
                });
            }
        }
    }
}

/// If `sqlite:` path is relative, resolve it against the directory of `sova.toml`.
fn resolve_sqlite_url(url: &str, source_dir: Option<&Path>) -> String {
    let Some(dir) = source_dir else {
        return url.to_string();
    };
    let rest = match url.strip_prefix("sqlite:") {
        Some(r) if !r.starts_with('/') && !r.starts_with("//") && r != ":memory:" && !r.starts_with(":memory:") => r,
        _ => return url.to_string(),
    };
    // `sqlite:hn.db?mode=rwc` or `sqlite:./data/x.db?mode=rwc`
    let (path_part, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    let path = Path::new(path_part);
    if path.is_absolute() {
        return url.to_string();
    }
    let abs = dir.join(path);
    match query {
        Some(q) => format!("sqlite://{}?{q}", abs.display()),
        None => format!("sqlite://{}", abs.display()),
    }
}
