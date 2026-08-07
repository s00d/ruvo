use crate::handle::DbPool;
use crate::tx::inject_conn;
use ruvo_core::{App, Error, Plugin};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::sync::Arc;

type MigrateFn = Arc<
    dyn Fn(
            DatabaseConnection,
            Vec<String>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Error>> + Send>>
        + Send
        + Sync,
>;

/// SeaORM pool plugin (backend selected by URL + Cargo features).
pub struct Db {
    url: String,
    migrate: Option<MigrateFn>,
}

impl Db {
    pub fn from_env() -> Self {
        let url = std::env::var("DATABASE_URL").unwrap_or_default();
        Self { url, migrate: None }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Register `myapp migrate` / `migrate status` / `migrate down` CLI hooks.
    pub fn migrations<M: MigratorTrait + 'static>(mut self) -> Self {
        self.migrate = Some(Arc::new(move |conn, args| {
            Box::pin(async move { run_migrate::<M>(conn, &args).await })
        }));
        self
    }
}

impl Plugin for Db {
    fn id(&self) -> &'static str {
        "db"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Database")
            .description("SeaORM pool and migrate CLI hooks")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        if self.url.is_empty() {
            app.on_startup(|_state| async {
                Err(Error::Internal(
                    "DATABASE_URL is empty; set it before installing Db".into(),
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
    }
}

async fn run_migrate<M: MigratorTrait>(
    conn: DatabaseConnection,
    args: &[String],
) -> Result<(), Error> {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "up" => M::up(&conn, None)
            .await
            .map_err(|e| Error::Internal(format!("migrate up: {e}")))?,
        "status" => M::status(&conn)
            .await
            .map_err(|e| Error::Internal(format!("migrate status: {e}")))?,
        "down" => M::down(&conn, Some(1))
            .await
            .map_err(|e| Error::Internal(format!("migrate down: {e}")))?,
        other => {
            return Err(Error::Internal(format!(
                "unknown migrate subcommand `{other}` (use up|status|down)"
            )));
        }
    }
    Ok(())
}
