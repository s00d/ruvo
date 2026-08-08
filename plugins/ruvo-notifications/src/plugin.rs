//! Notifications plugin: state, HTTP mount, optional WS / template helpers.

use crate::channel::Channel;
use crate::http::mount_routes;
use crate::notify::NotificationService;
use ruvo_core::extend::{IntoMwEntry, MwEntry};
use ruvo_core::{App, Plugin, Router};
use std::collections::HashMap;
use std::sync::Arc;

/// Database notifications with named channels.
pub struct Notifications {
    channels: HashMap<String, Channel>,
    mount: Option<String>,
    guard: Option<MwEntry>,
    #[cfg(feature = "ws")]
    ws_path: Option<String>,
    #[cfg(feature = "templates")]
    template_helpers: bool,
}

impl Notifications {
    pub fn new() -> Self {
        let mut channels = HashMap::new();
        channels.insert("default".into(), Channel::new("default"));
        Self {
            channels,
            mount: None,
            guard: None,
            #[cfg(feature = "ws")]
            ws_path: None,
            #[cfg(feature = "templates")]
            template_helpers: false,
        }
    }

    pub fn channel(mut self, ch: Channel) -> Self {
        self.channels.insert(ch.slug.clone(), ch);
        self
    }

    /// Inbox API under `path` (`GET /`, unread-count, mark read, broadcast).
    pub fn mount(mut self, path: impl Into<String>) -> Self {
        self.mount = Some(path.into());
        self
    }

    pub fn guard(mut self, mw: impl IntoMwEntry) -> Self {
        self.guard = Some(mw.into_mw_entry());
        self
    }

    /// WebSocket path (`join user:{id}`); requires installed `Ws` + feature `ws`.
    #[cfg(feature = "ws")]
    pub fn ws_path(mut self, path: impl Into<String>) -> Self {
        self.ws_path = Some(path.into());
        self
    }

    /// Register Minijinja `notifications_unread` per-request helper.
    #[cfg(feature = "templates")]
    pub fn with_template_helpers(mut self) -> Self {
        self.template_helpers = true;
        self
    }
}

impl Default for Notifications {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Notifications {
    fn id(&self) -> &'static str {
        "notifications"
    }

    fn requires(&self) -> &'static [&'static str] {
        &["db"]
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Notifications")
            .description("DB inbox, channels with ACL, optional WS/mail")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.state(NotificationService {
            channels: Arc::new(self.channels),
        });

        let guard = self.guard;

        if let Some(path) = self.mount {
            let mut r = Router::new();
            if let Some(ref g) = guard {
                r.use_middleware(g.clone());
            }
            #[cfg(feature = "templates")]
            if self.template_helpers {
                r.use_middleware(crate::templates::preload_middleware());
            }
            mount_routes(&mut r);
            app.mount(&path, r);
        }

        #[cfg(feature = "ws")]
        if let Some(path) = self.ws_path {
            crate::ws::install_ws(app, &path, guard);
        }

        #[cfg(feature = "templates")]
        if self.template_helpers {
            crate::templates::register(app);
        }
    }
}
