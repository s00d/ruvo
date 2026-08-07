//! Facade plugin wrapping [`ruvo_store::AppStore`].

use ruvo_core::{App, Plugin};
use ruvo_store::{AppStore, KvStore};
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

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Shared store")
            .description("Install AppStore for namespaced KvStore access")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.state(self.0);
    }
}
