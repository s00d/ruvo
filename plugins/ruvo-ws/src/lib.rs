//! WebSocket plugin for Ruvo (HTTP upgrade + rooms hub).

mod ext;
mod hub;
mod upgrade;

pub use ext::WsRouteExt;
pub use hub::{Hub, RoomHandle};
pub use tokio_tungstenite::tungstenite::Message;
pub use upgrade::{origin_allowed, upgrade_ws, WsSession};

use std::sync::Arc;

use ruvo_core::{App, Plugin};

/// WebSocket plugin configuration.
#[derive(Clone, Default)]
pub struct Ws {
    origins: Vec<String>,
    max_message_size: Option<usize>,
}

/// Shared state installed by [`Ws`].
#[derive(Clone)]
pub struct WsShared {
    pub hub: Hub,
    pub config: Arc<WsConfig>,
}

#[derive(Clone)]
pub struct WsConfig {
    pub origins: Vec<String>,
    pub max_message_size: Option<usize>,
}

impl Ws {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allowed `Origin` values (CSWSH). Empty → allow all (dev).
    pub fn origins(
        mut self,
        origins: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.origins = origins.into_iter().map(Into::into).collect();
        self
    }

    pub fn max_message_size(mut self, n: usize) -> Self {
        self.max_message_size = Some(n);
        self
    }
}

impl Plugin for Ws {
    fn id(&self) -> &'static str {
        "ws"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("WebSocket")
            .description("WebSocket hub, origin allowlist, max message size")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.state(WsShared {
            hub: Hub::new(),
            config: Arc::new(WsConfig {
                origins: self.origins,
                max_message_size: self.max_message_size,
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderMap;

    #[test]
    fn origin_empty_allowlist_allows_all() {
        let mut headers = HeaderMap::new();
        headers.insert("origin", "https://evil.test".parse().unwrap());
        assert!(origin_allowed(&headers, &[]));
    }

    #[test]
    fn origin_rejects_unknown() {
        let mut headers = HeaderMap::new();
        headers.insert("origin", "https://evil.test".parse().unwrap());
        assert!(!origin_allowed(
            &headers,
            &["https://good.test".to_string()]
        ));
    }

    #[test]
    fn origin_accepts_match() {
        let mut headers = HeaderMap::new();
        headers.insert("origin", "https://good.test".parse().unwrap());
        assert!(origin_allowed(
            &headers,
            &["https://good.test".to_string()]
        ));
    }
}
