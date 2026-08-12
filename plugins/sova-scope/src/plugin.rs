//! ScopeAuth plugin install.

use crate::engine::{AuthorizeEngine, SharedEngine};
use crate::matrix::MatrixBuilder;
use crate::owner::OwnerRegistry;
use sova_core::{App, Plugin};
use sova_db::DbHandle;
use std::future::Future;
use std::sync::Arc;

type ConfigureFn = Box<dyn FnOnce(&mut MatrixBuilder) + Send>;

pub struct ScopeAuthState {
    pub engine: SharedEngine,
}

pub struct ScopeAuth {
    configure_fn: Option<ConfigureFn>,
    owners: OwnerRegistry,
}

impl ScopeAuth {
    pub fn new() -> Self {
        Self {
            configure_fn: None,
            owners: OwnerRegistry::default(),
        }
    }

    pub fn configure(mut self, f: impl FnOnce(&mut MatrixBuilder) + Send + 'static) -> Self {
        self.configure_fn = Some(Box::new(f));
        self
    }

    pub fn owner<F, Fut>(mut self, kind: &'static str, f: F) -> Self
    where
        F: Fn(DbHandle, i64) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = sova_core::Result<Option<i64>>> + Send + 'static,
    {
        self.owners.register(kind, crate::owner::callback_owner(f));
        self
    }
}

impl Default for ScopeAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ScopeAuth {
    fn id(&self) -> &'static str {
        "scope"
    }

    fn requires(&self) -> &'static [&'static str] {
        &["db"]
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("ScopeAuth")
            .description("Scoped RBAC: membership, role matrix, ownership hooks, cache, audit")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        let mut builder = MatrixBuilder::default();
        if let Some(f) = self.configure_fn {
            f(&mut builder);
        }
        let matrices = builder.build();
        let engine = Arc::new(AuthorizeEngine::new(matrices, self.owners));
        app.state(ScopeAuthState { engine });
    }
}
