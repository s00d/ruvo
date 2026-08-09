//! Facade plugin wrapping [`sova_store::AppStore`].

use sova_core::{App, Plugin};
use sova_store::{AppStore, KvStore};
use std::sync::Arc;

/// Install a shared [`AppStore`] for session / meta / rate-limit namespaces.
pub struct SharedStore(pub AppStore);

impl SharedStore {
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self(AppStore::new(store))
    }

    pub fn memory() -> Self {
        Self(AppStore::memory())
    }

    /// SQL KvStore — requires [`sova_db::Db`] installed first.
    #[cfg(feature = "store-sql")]
    pub fn sql(app: &App) -> Self {
        let pool = app.try_state::<sova_db::DbPool>().unwrap_or_else(|| {
            panic!("SharedStore::sql requires Db plugin installed first")
        });
        Self::new(Arc::new(sova_store::SqlStore::from_db_pool(pool.as_ref())))
    }

    /// Redis KvStore — requires [`sova_redis::Redis`] installed first.
    #[cfg(feature = "store-redis")]
    pub fn redis(app: &App) -> Self {
        let pool = app.try_state::<sova_redis::RedisPool>().unwrap_or_else(|| {
            panic!("SharedStore::redis requires Redis plugin installed first")
        });
        Self::new(Arc::new(sova_store::RedisStore::from_redis_pool(
            pool.as_ref(),
        )))
    }

    /// Embedded redb KvStore at `path` (creates parent dirs / file).
    #[cfg(feature = "store-redb")]
    pub fn redb(path: impl AsRef<std::path::Path>) -> Self {
        let path = path.as_ref();
        let store = sova_store::RedbStore::open(path).unwrap_or_else(|e| {
            panic!(
                "SharedStore::redb failed to open {}: {e}",
                path.display()
            )
        });
        Self::new(Arc::new(store))
    }
}

impl Plugin for SharedStore {
    fn id(&self) -> &'static str {
        "store"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Shared store")
            .description("Install AppStore for namespaced KvStore access")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.state(self.0);
    }
}
