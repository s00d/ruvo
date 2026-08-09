//! Tempfile SQLite database for integration tests.

use sova_db::DbHandle;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tempfile::TempDir;

/// Isolated sqlite file + URL. Keeps [`TempDir`] alive for the test duration.
pub struct SqliteTestDb {
    _dir: TempDir,
    url: String,
}

impl SqliteTestDb {
    /// Create an empty sqlite file and set `DATABASE_URL`.
    pub fn create() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        // SAFETY: test-only; integration tests own the process env for DB URL.
        std::env::set_var("DATABASE_URL", &url);
        Self { _dir: dir, url }
    }

    /// Create + apply all migrations from `M` (see [`apply_migrations`]).
    pub async fn migrate<M: MigratorTrait>() -> Self {
        let db = Self::create();
        apply_migrations::<M>(&db.url).await;
        db
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Fresh connection (e.g. after TestClient owns the App pool).
    pub async fn connect(&self) -> DatabaseConnection {
        Database::connect(&self.url)
            .await
            .expect("sqlite reconnect")
    }

    /// [`DbHandle`] over a new connection.
    pub async fn handle(&self) -> DbHandle {
        DbHandle::Conn(self.connect().await)
    }
}

/// Run each migration's `up` (for tests). Prefer unique [`MigrationName`]s so
/// `MigratorTrait::up` / `migrate` CLI also work on composed migrators.
pub async fn apply_migrations<M: MigratorTrait>(url: &str) {
    use sea_orm_migration::SchemaManager;
    let conn = Database::connect(url).await.expect("sqlite connect");
    let schema = SchemaManager::new(&conn);
    for m in M::migrations() {
        m.up(&schema).await.expect("migrate up");
    }
}
