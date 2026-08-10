//! Session bag console — get/set/del/clear/regenerate on the current browser session.

use super::{json_response, ActionResponse, MAX_BODY};
use crate::console::DevToolsConsole;
use serde::Deserialize;
use serde_json::json;
use sova_core::{Request, Response};
use sova_session::SessionExt;
use std::collections::BTreeMap;
use std::time::Instant;

#[derive(Debug, Deserialize)]
pub struct SessionActionRequest {
    pub op: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
}

fn snapshot(sess: &sova_session::Session) -> serde_json::Value {
    let mut keys: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in sess.data() {
        keys.insert(k, v);
    }
    json!({
        "session_id": sess.id(),
        "user_id": sess.user_id(),
        "keys": keys,
    })
}

pub async fn handle(req: &mut Request, cfg: &DevToolsConsole) -> Response {
    let started = Instant::now();
    let body: SessionActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(ActionResponse::err(format!("invalid JSON: {e}"), 0.0), 400);
        }
    };

    let sess = req.session();
    let ms = || started.elapsed().as_secs_f64() * 1000.0;

    match body.op.to_ascii_lowercase().as_str() {
        "list" | "get" => json_response(ActionResponse::ok(snapshot(&sess), ms()), 200),
        "set" => {
            let key = match body.key.filter(|k| !k.is_empty()) {
                Some(k) => k,
                None => return json_response(ActionResponse::err("key required", ms()), 400),
            };
            let value = body.value.unwrap_or_default();
            if value.len() > cfg.body_limit.min(MAX_BODY) {
                return json_response(ActionResponse::err("value too large", ms()), 400);
            }
            sess.set(key, value);
            json_response(ActionResponse::ok(snapshot(&sess), ms()), 200)
        }
        "del" | "delete" => {
            let key = match body.key.filter(|k| !k.is_empty()) {
                Some(k) => k,
                None => return json_response(ActionResponse::err("key required", ms()), 400),
            };
            sess.remove(&key);
            json_response(ActionResponse::ok(snapshot(&sess), ms()), 200)
        }
        "clear" => {
            sess.clear();
            json_response(ActionResponse::ok(snapshot(&sess), ms()), 200)
        }
        "regenerate" => {
            sess.regenerate();
            json_response(ActionResponse::ok(snapshot(&sess), ms()), 200)
        }
        "destroy" => {
            sess.destroy();
            json_response(ActionResponse::ok(json!({ "destroyed": true }), ms()), 200)
        }
        "bind_user" => {
            let uid = match body.user_id.filter(|s| !s.is_empty()) {
                Some(u) => u,
                None => return json_response(ActionResponse::err("user_id required", ms()), 400),
            };
            sess.bind_user(uid);
            json_response(ActionResponse::ok(snapshot(&sess), ms()), 200)
        }
        other => json_response(
            ActionResponse::err(format!("unknown op: {other}"), ms()),
            400,
        ),
    }
}
