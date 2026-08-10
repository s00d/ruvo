//! Task enqueue console action.

use super::{json_response, ActionResponse};
use serde::Deserialize;
use serde_json::Value;
use sova_core::{Request, Response};
use sova_tasks::{Dispatch, TaskBackend};
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
pub struct TasksActionRequest {
    pub name: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub queue: Option<String>,
    #[serde(default)]
    pub delay_secs: Option<u64>,
    #[serde(default)]
    pub dedup_key: Option<String>,
    #[serde(default)]
    pub priority: Option<i32>,
}

pub async fn handle(req: &mut Request) -> Response {
    let started = Instant::now();
    let body: TasksActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(ActionResponse::err(format!("invalid JSON: {e}"), 0.0), 400);
        }
    };

    if body.name.trim().is_empty() {
        return json_response(ActionResponse::err("name required", 0.0), 400);
    }

    let backend = match req.try_state::<TaskBackend>() {
        Some(b) => b,
        None => {
            return json_response(ActionResponse::err("Tasks not installed", 0.0), 503);
        }
    };

    let mut d = Dispatch::new(body.name.trim()).data(body.payload);
    if let Some(q) = body.queue {
        d = d.queue(q);
    }
    if let Some(secs) = body.delay_secs {
        d = d.delay(Duration::from_secs(secs));
    }
    if let Some(k) = body.dedup_key {
        d = d.dedup(k);
    }
    if let Some(p) = body.priority {
        d = d.priority(p);
    }

    match backend.dispatch(d).await {
        Ok(id) => {
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            json_response(ActionResponse::ok(serde_json::json!({ "id": id }), ms), 200)
        }
        Err(e) => json_response(
            ActionResponse::err(e.to_string(), started.elapsed().as_secs_f64() * 1000.0),
            400,
        ),
    }
}
