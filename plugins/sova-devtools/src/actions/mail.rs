//! Mail console — fake transport compose / list / clear.

use super::{json_response, ActionResponse, MAX_BODY};
use crate::console::DevToolsConsole;
use serde::Deserialize;
use serde_json::{json, Value};
use sova_core::{Request, Response};
use sova_mail::{MailClient, MailExt};
use std::time::Instant;

#[derive(Debug, Deserialize)]
pub struct MailActionRequest {
    pub op: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
}

pub async fn handle(req: &mut Request, cfg: &DevToolsConsole) -> Response {
    let started = Instant::now();
    let body: MailActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(ActionResponse::err(format!("invalid JSON: {e}"), 0.0), 400);
        }
    };

    let client = match req.try_state::<MailClient>() {
        Some(c) => c,
        None => {
            return json_response(ActionResponse::err("Mail not installed", 0.0), 503);
        }
    };

    let ms = || started.elapsed().as_secs_f64() * 1000.0;

    match body.op.to_ascii_lowercase().as_str() {
        "send" => {
            if !cfg.allow_dangerous && client.fake().is_none() {
                return json_response(
                    ActionResponse::err("only fake mail backend allowed", ms()),
                    403,
                );
            }
            let to = body
                .to
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "dev@localhost".into());
            let subject = body.subject.unwrap_or_else(|| "DevTools test".into());
            let text = body.body.unwrap_or_default();
            if text.len() > cfg.body_limit.min(MAX_BODY) {
                return json_response(ActionResponse::err("body too large", ms()), 400);
            }
            match req.mail().to(to).subject(subject).text(text).send().await {
                Ok(()) => json_response(ActionResponse::ok(json!({ "ok": true }), ms()), 200),
                Err(e) => json_response(ActionResponse::err(e.to_string(), ms()), 400),
            }
        }
        "list" => {
            let fake = match client.fake() {
                Some(f) => f,
                None => {
                    return json_response(
                        ActionResponse::err("fake mail only for list", ms()),
                        403,
                    );
                }
            };
            let items: Vec<Value> = fake
                .sent()
                .into_iter()
                .map(|m| {
                    json!({
                        "to": m.to,
                        "subject": m.subject,
                        "text": m.text,
                    })
                })
                .collect();
            json_response(ActionResponse::ok(json!({ "messages": items }), ms()), 200)
        }
        "clear" => {
            let fake = match client.fake() {
                Some(f) => f,
                None => {
                    return json_response(
                        ActionResponse::err("fake mail only for clear", ms()),
                        403,
                    );
                }
            };
            fake.clear();
            json_response(ActionResponse::ok(json!({ "ok": true }), ms()), 200)
        }
        other => json_response(
            ActionResponse::err(format!("unknown op `{other}`"), ms()),
            400,
        ),
    }
}
