//! Shared runtime config fragments for DevTools (mount paths, plugin metadata).

use serde_json::{Map, Value};
use std::sync::{Arc, Mutex};

/// Plugins push JSON blobs here; DevTools merges them into `GET /_devtools/config`.
#[derive(Clone, Default)]
pub struct DevToolsConfigRegistry(Arc<Mutex<Map<String, Value>>>);

impl DevToolsConfigRegistry {
    pub fn set(&self, key: impl Into<String>, value: Value) {
        self.0.lock().unwrap().insert(key.into(), value);
    }

    pub fn snapshot(&self) -> Map<String, Value> {
        self.0.lock().unwrap().clone()
    }
}
