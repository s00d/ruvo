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
