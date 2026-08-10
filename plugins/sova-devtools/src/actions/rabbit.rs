//! RabbitMQ publish console action.

use super::{json_response, ActionResponse, MAX_BODY};
use crate::console::DevToolsConsole;
use bytes::Bytes;
use serde::Deserialize;
use serde_json::json;
use sova_core::{Request, Response};
use sova_rabbit::{Exchange, SharedBroker};
use std::time::Instant;

#[derive(Debug, Deserialize)]
pub struct RabbitActionRequest {
    pub op: String,
    #[serde(default)]
    pub exchange: Option<String>,
    #[serde(default)]
    pub routing_key: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub queue: Option<String>,
}

pub async fn handle(req: &mut Request, cfg: &DevToolsConsole) -> Response {
    let started = Instant::now();
    let body: RabbitActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(ActionResponse::err(format!("invalid JSON: {e}"), 0.0), 400);
        }
    };

    let broker = match req.try_state::<SharedBroker>() {
        Some(b) => b,
        None => {
            return json_response(ActionResponse::err("Rabbit not installed", 0.0), 503);
        }
    };

    let ms = || started.elapsed().as_secs_f64() * 1000.0;

    match body.op.to_ascii_lowercase().as_str() {
        "publish" => {
            let exchange = body.exchange.unwrap_or_else(|| "amq.direct".into());
            let routing_key = body.routing_key.unwrap_or_default();
            let payload = body.body.unwrap_or_default();
            if payload.len() > cfg.body_limit.min(MAX_BODY) {
                return json_response(ActionResponse::err("body too large", ms()), 400);
            }
            match broker
                .publish(
                    &Exchange::direct(&exchange),
                    &routing_key,
                    Bytes::from(payload),
                )
                .await
            {
                Ok(()) => json_response(ActionResponse::ok(json!({ "ok": true }), ms()), 200),
                Err(e) => json_response(ActionResponse::err(e.to_string(), ms()), 400),
            }
        }
        "consume_one" => {
            let queue = body
                .queue
                .filter(|q| !q.is_empty())
                .ok_or_else(|| "queue required".to_string());
            let queue = match queue {
                Ok(q) => q,
                Err(e) => return json_response(ActionResponse::err(e, ms()), 400),
            };
            match broker.consume_one(&queue).await {
                Ok(Some(d)) => {
                    let body = String::from_utf8_lossy(&d.body).into_owned();
                    let routing_key = d.routing_key.clone();
                    let _ = d.ack().await;
                    json_response(
                        ActionResponse::ok(
                            json!({
                                "body": body,
                                "routing_key": routing_key,
                            }),
                            ms(),
                        ),
                        200,
                    )
                }
                Ok(None) => json_response(ActionResponse::ok(json!({ "empty": true }), ms()), 200),
                Err(e) => json_response(ActionResponse::err(e.to_string(), ms()), 400),
            }
        }
        other => json_response(
            ActionResponse::err(format!("unknown op `{other}`"), ms()),
            400,
        ),
    }
}
