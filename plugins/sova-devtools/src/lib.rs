//! In-app DevTools for Sova — thin host bridge + SPA panel.
//!
//! Enable only in development (`[development.devtools] enabled = true` or
//! `SOVA_DEVTOOLS=1`). Injects into `text/html` responses only.
//!
//! Host pages get a tiny `bridge.js` (bar + dock iframe). The Vue app
//! is served at `/_devtools/app`. Bar toggles the dock iframe; "New tab"
//! only opens the SPA in a browser tab (no persisted mode).

//!
//! Frontend: `ui/` (Vite + Vue + Tailwind + Pinia + Vue Router + TS).
//! Build: `npm --prefix ui ci && npm --prefix ui run build` → `assets/`.

#[cfg(feature = "console")]
mod actions;
mod collector;
mod console;
mod hooks;
mod hub;
mod inject;
mod middleware;
mod plugin;
mod redact;
mod routes;

pub use collector::{DevToolsBag, HttpLine, LogLine, QueryLine, RequestSnapshot};
pub use hub::{CustomEvent, DevToolsHub, MemorySample, MemorySummary};
pub use inject::{inject_body, DEVTOOLS_MARKER};
pub use plugin::DevTools;

pub(crate) static APP_CSS: &str = include_str!("../assets/app.css");
pub(crate) static APP_JS: &str = include_str!("../assets/app.js");
pub(crate) static BRIDGE_JS: &str = include_str!("../assets/bridge.js");
pub(crate) static SHELL_HTML: &str = include_str!("../assets/index.html");
