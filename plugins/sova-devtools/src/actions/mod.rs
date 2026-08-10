//! DevTools control panel actions (POST).

#[cfg(feature = "console-events")]
mod events;
#[cfg(feature = "console-graphql")]
mod graphql;
#[cfg(feature = "console-grpc")]
mod grpc;
mod http;
#[cfg(feature = "console-mail")]
mod mail;
#[cfg(feature = "console-rabbit")]
mod rabbit;
#[cfg(feature = "console-redis")]
mod redis;
#[cfg(feature = "console-session")]
mod session;
#[cfg(feature = "console-store")]
mod store;
#[cfg(feature = "console-tasks")]
mod tasks;

use crate::console::DevToolsConsole;
use crate::hub::DevToolsHub;
use serde::Serialize;
use serde_json::{json, Value};
use sova_core::{Request, Response};
use std::time::Instant;

pub const MAX_BODY: usize = 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct ActionResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    pub duration_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ActionResponse {
    fn ok(result: Value, ms: f64) -> Self {
        Self {
            ok: true,
            result: Some(result),
            duration_ms: ms,
            error: None,
        }
    }

    fn err(msg: impl Into<String>, ms: f64) -> Self {
        Self {
            ok: false,
            result: None,
            duration_ms: ms,
            error: Some(msg.into()),
        }
    }
}

pub(crate) fn json_response(res: ActionResponse, status: u16) -> Response {
    Response::json(&res).status(status)
}

pub(crate) fn mount(app: &mut sova_core::App, hub: DevToolsHub, console: DevToolsConsole) {
    if !console.enabled {
        return;
    }

    let hub_http = hub.clone();
    let cfg_http = console.clone();
    app.post("/_devtools/actions/http", move |mut req: Request| {
        let hub = hub_http.clone();
        let cfg = cfg_http.clone();
        async move { handle_http(&mut req, &hub, &cfg).await }
    });

    #[cfg(feature = "console-redis")]
    {
        let hub_redis = hub.clone();
        let cfg_redis = console.clone();
        app.post("/_devtools/actions/redis", move |mut req: Request| {
            let hub = hub_redis.clone();
            let cfg = cfg_redis.clone();
            async move { redis::handle(&mut req, &hub, &cfg).await }
        });

        let hub_stream = hub.clone();
        let cfg_stream = console.clone();
        app.get("/_devtools/stream/redis", move |req: Request| {
            let hub = hub_stream.clone();
            let cfg = cfg_stream.clone();
            async move { redis::stream_subscribe(req, &hub, &cfg).await }
        });
    }

    #[cfg(feature = "console-store")]
    {
        let cfg_store = console.clone();
        app.post("/_devtools/actions/store", move |mut req: Request| {
            let cfg = cfg_store.clone();
            async move { store::handle(&mut req, &cfg).await }
        });
    }

    #[cfg(feature = "console-graphql")]
    {
        app.post(
            "/_devtools/actions/graphql",
            move |mut req: Request| async move { graphql::handle(&mut req).await },
        );
    }

    #[cfg(feature = "console-tasks")]
    {
        app.post(
            "/_devtools/actions/tasks",
            move |mut req: Request| async move { tasks::handle(&mut req).await },
        );
    }

    #[cfg(feature = "console-mail")]
    {
        let cfg_mail = console.clone();
        app.post("/_devtools/actions/mail", move |mut req: Request| {
            let cfg = cfg_mail.clone();
            async move { mail::handle(&mut req, &cfg).await }
        });
    }

    #[cfg(feature = "console-events")]
    {
        let hub_events = hub.clone();
        app.post("/_devtools/actions/events", move |mut req: Request| {
            let hub = hub_events.clone();
            async move { events::handle(&mut req, &hub).await }
        });
    }

    #[cfg(feature = "console-rabbit")]
    {
        let cfg_rabbit = console.clone();
        app.post("/_devtools/actions/rabbit", move |mut req: Request| {
            let cfg = cfg_rabbit.clone();
            async move { rabbit::handle(&mut req, &cfg).await }
        });
    }

    #[cfg(feature = "console-grpc")]
    {
        app.post(
            "/_devtools/actions/grpc",
            move |mut req: Request| async move { grpc::handle(&mut req).await },
        );
    }

    #[cfg(feature = "console-session")]
    {
        let cfg_sess = console.clone();
        app.post("/_devtools/actions/session", move |mut req: Request| {
            let cfg = cfg_sess.clone();
            async move { session::handle(&mut req, &cfg).await }
        });
    }

    let _ = console;
}

async fn handle_http(req: &mut Request, hub: &DevToolsHub, cfg: &DevToolsConsole) -> Response {
    let started = Instant::now();
    let body: http::HttpActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                ActionResponse::err(
                    format!("invalid JSON: {e}"),
                    started.elapsed().as_secs_f64() * 1000.0,
                ),
                400,
            );
        }
    };

    let audit = json!({
        "domain": "http",
        "method": body.method,
        "path": body.path,
        "target": body.target,
    });

    match http::execute(req, cfg, body).await {
        Ok(result) => {
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            hub.emit(
                "devtools.action",
                json!({ "domain": "http", "ok": true, "detail": audit }),
            );
            json_response(ActionResponse::ok(result, ms), 200)
        }
        Err(e) => {
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            hub.emit(
                "devtools.action",
                json!({ "domain": "http", "ok": false, "error": e, "detail": audit }),
            );
            json_response(ActionResponse::err(e, ms), 400)
        }
    }
}

pub(crate) fn truncate_body(bytes: &[u8], limit: usize) -> (String, bool) {
    if bytes.len() <= limit {
        return (String::from_utf8_lossy(bytes).into_owned(), false);
    }
    let slice = &bytes[..limit.min(bytes.len())];
    (
        format!(
            "{}… [truncated {} bytes]",
            String::from_utf8_lossy(slice),
            bytes.len() - slice.len()
        ),
        true,
    )
}
