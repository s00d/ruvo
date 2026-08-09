//! `/_devtools/*` JSON + SSE + SPA shell + assets.

use crate::hub::DevToolsHub;
use crate::{APP_CSS, APP_JS, BRIDGE_JS, SHELL_HTML};
use sova_core::{Request, Response};
use sova_sse::sse_response;
use std::sync::Arc;
use std::time::Duration;

pub fn mount(app: &mut sova_core::App, hub: DevToolsHub) {
    let hub_events = hub.clone();
    app.get("/_devtools/events", move |req: Request| {
        let hub = hub_events.clone();
        async move { sse_response(&req, &hub.channel, Duration::from_secs(15)) }
    });

    let hub_list = hub.clone();
    app.get("/_devtools/requests", move |_req: Request| {
        let hub = hub_list.clone();
        async move {
            let list = hub.list_meta(100);
            Response::json(&list)
        }
    });

    let hub_one = hub.clone();
    app.get("/_devtools/requests/:id", move |req: Request| {
        let hub = hub_one.clone();
        async move {
            let id = req.param("id").unwrap_or("");
            match hub.get(id) {
                Some(snap) => Response::json(&snap),
                None => Response::text("not found").status(404),
            }
        }
    });

    let hub_logs = hub.clone();
    app.get("/_devtools/logs", move |_req: Request| {
        let hub = hub_logs.clone();
        async move { Response::json(&hub.recent_logs(200)) }
    });

    let hub_cfg = hub.clone();
    app.get("/_devtools/config", move |_req: Request| {
        let hub = hub_cfg.clone();
        async move { Response::json(&hub.config_json()) }
    });

    app.get("/_devtools/app", |_req: Request| async move {
        Response::text(SHELL_HTML).header("content-type", "text/html; charset=utf-8")
    });

    app.get("/_devtools/assets/app.css", |_req: Request| async move {
        Response::text(APP_CSS).header("content-type", "text/css; charset=utf-8")
    });

    app.get("/_devtools/assets/app.js", |_req: Request| async move {
        Response::text(APP_JS)
            .header("content-type", "application/javascript; charset=utf-8")
    });

    app.get("/_devtools/assets/bridge.js", |_req: Request| async move {
        Response::text(BRIDGE_JS)
            .header("content-type", "application/javascript; charset=utf-8")
    });

    let _ = Arc::new(hub);
}
