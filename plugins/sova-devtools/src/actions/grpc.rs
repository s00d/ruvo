//! gRPC console — in-process Connect-JSON unary call via [`GrpcClient`].

use super::{json_response, truncate_body, ActionResponse, MAX_BODY};
use bytes::Bytes;
use serde::Deserialize;
use serde_json::{json, Value};
use sova_core::{Request, Response};
use sova_grpc::GrpcClient;
use std::time::Instant;

#[derive(Debug, Deserialize)]
pub struct GrpcActionRequest {
    pub method: String,
    #[serde(default)]
    pub body: Option<Value>,
}

pub async fn handle(req: &mut Request) -> Response {
    let started = Instant::now();
    let body: GrpcActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(ActionResponse::err(format!("invalid JSON: {e}"), 0.0), 400);
        }
    };

    if body.method.trim().is_empty() {
        return json_response(ActionResponse::err("method required", 0.0), 400);
    }

    let client = match req.try_state::<GrpcClient>() {
        Some(c) => c,
        None => {
            return json_response(ActionResponse::err("gRPC client not installed", 0.0), 503);
        }
    };

    let payload = match body.body {
        Some(v) => serde_json::to_vec(&v).unwrap_or_else(|_| b"{}".to_vec()),
        None => b"{}".to_vec(),
    };
    if payload.len() > MAX_BODY {
        return json_response(
            ActionResponse::err(format!("body exceeds {MAX_BODY} bytes"), 0.0),
            413,
        );
    }

    let method = body.method.trim().to_string();
    let bytes_in = payload.len();
    match client.call_raw(&method, Bytes::from(payload)).await {
        Ok(out) => {
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            let (body_text, truncated) = truncate_body(&out, MAX_BODY);
            let parsed: Value = serde_json::from_slice(&out).unwrap_or(json!(body_text));
            json_response(
                ActionResponse::ok(
                    json!({
                        "method": method,
                        "base": client.base(),
                        "status": 200,
                        "body": parsed,
                        "bytes_in": bytes_in,
                        "bytes_out": out.len(),
                        "truncated": truncated,
                    }),
                    ms,
                ),
                200,
            )
        }
        Err(e) => {
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            json_response(ActionResponse::err(e.to_string(), ms), 400)
        }
    }
}
