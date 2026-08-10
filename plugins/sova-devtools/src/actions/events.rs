//! Emit custom DevTools hub events from the UI.

use super::{json_response, ActionResponse};
use crate::hub::DevToolsHub;
use serde::Deserialize;
use serde_json::Value;
use sova_core::{Request, Response};
use std::time::Instant;

#[derive(Debug, Deserialize)]
pub struct EventsActionRequest {
    pub kind: String,
    #[serde(default)]
    pub payload: Value,
}

pub async fn handle(req: &mut Request, hub: &DevToolsHub) -> Response {
    let started = Instant::now();
    let body: EventsActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(ActionResponse::err(format!("invalid JSON: {e}"), 0.0), 400);
        }
    };

    if body.kind.trim().is_empty() {
        return json_response(ActionResponse::err("kind required", 0.0), 400);
    }

    hub.emit(body.kind.trim(), body.payload);
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    json_response(
        ActionResponse::ok(serde_json::json!({ "ok": true }), ms),
        200,
    )
}
