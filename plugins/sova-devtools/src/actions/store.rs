//! AppStore / KvStore console actions.

use super::{json_response, ActionResponse, MAX_BODY};
use crate::console::DevToolsConsole;
use bytes::Bytes;
use serde::Deserialize;
use serde_json::json;
use sova_core::{Request, Response};
use sova_store::AppStore;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
pub struct StoreActionRequest {
    pub namespace: String,
    pub op: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub ttl_secs: Option<u64>,
    #[serde(default)]
    pub by: Option<i64>,
    #[serde(default)]
    pub prefix: Option<String>,
}

pub async fn handle(req: &mut Request, cfg: &DevToolsConsole) -> Response {
    let started = Instant::now();
    let body: StoreActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(ActionResponse::err(format!("invalid JSON: {e}"), 0.0), 400);
        }
    };

    if body.namespace.trim().is_empty() {
        return json_response(ActionResponse::err("namespace required", 0.0), 400);
    }

    let store = match req.try_state::<AppStore>() {
        Some(s) => s,
        None => {
            return json_response(ActionResponse::err("AppStore not installed", 0.0), 503);
        }
    };

    let ns = store.namespaced(body.namespace.trim());
    let ms = || started.elapsed().as_secs_f64() * 1000.0;

    match body.op.to_ascii_lowercase().as_str() {
        "get" => {
            let Some(key) = body.key.filter(|k| !k.is_empty()) else {
                return json_response(ActionResponse::err("key required", ms()), 400);
            };
            let val = ns.get(&key).await;
            let text = val
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            json_response(
                ActionResponse::ok(
                    json!({ "value": text, "bytes": val.map(|b| b.len()) }),
                    ms(),
                ),
                200,
            )
        }
        "set" => {
            let key = match body.key.filter(|k| !k.is_empty()) {
                Some(k) => k,
                None => return json_response(ActionResponse::err("key required", ms()), 400),
            };
            let value = body.value.unwrap_or_default();
            if value.len() > cfg.body_limit.min(MAX_BODY) {
                return json_response(ActionResponse::err("value too large", ms()), 400);
            }
            let ttl = body.ttl_secs.map(Duration::from_secs);
            ns.set(&key, Bytes::from(value), ttl).await;
            json_response(ActionResponse::ok(json!({ "ok": true }), ms()), 200)
        }
        "del" => {
            let key = match body.key.filter(|k| !k.is_empty()) {
                Some(k) => k,
                None => return json_response(ActionResponse::err("key required", ms()), 400),
            };
            ns.remove(&key).await;
            json_response(ActionResponse::ok(json!({ "ok": true }), ms()), 200)
        }
        "incr" => {
            let key = match body.key.filter(|k| !k.is_empty()) {
                Some(k) => k,
                None => return json_response(ActionResponse::err("key required", ms()), 400),
            };
            let by = body.by.unwrap_or(1);
            let ttl = body.ttl_secs.map(Duration::from_secs);
            let n = ns.incr(&key, by, ttl).await;
            json_response(ActionResponse::ok(json!({ "value": n }), ms()), 200)
        }
        "clear_prefix" => {
            let prefix = body.prefix.unwrap_or_default();
            if prefix.trim().is_empty() {
                return json_response(ActionResponse::err("prefix required (safety)", ms()), 403);
            }
            let n = ns.clear_prefix(prefix.trim()).await;
            json_response(ActionResponse::ok(json!({ "removed": n }), ms()), 200)
        }
        other => json_response(
            ActionResponse::err(format!("unknown op `{other}`"), ms()),
            400,
        ),
    }
}
