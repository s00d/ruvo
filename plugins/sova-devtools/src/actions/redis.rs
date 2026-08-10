//! Redis console actions + Pub/Sub SSE stream.

use super::{json_response, ActionResponse, MAX_BODY};
use crate::console::DevToolsConsole;
use crate::hub::DevToolsHub;
use redis::AsyncCommands;
use serde::Deserialize;
use serde_json::{json, Value};
use sova_core::{Request, Response};
use sova_redis::RedisPool;
use sova_sse::{sse_response, SseChannel, SseEvent};
use std::time::{Duration, Instant};

const DENYLIST: &[&str] = &[
    "FLUSHALL",
    "FLUSHDB",
    "CONFIG",
    "DEBUG",
    "SHUTDOWN",
    "SLAVEOF",
    "REPLICAOF",
];

#[derive(Debug, Deserialize)]
pub struct RedisActionRequest {
    pub op: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub db: Option<u8>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub cursor: Option<u64>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub ttl_secs: Option<u64>,
}

pub async fn handle(req: &mut Request, hub: &DevToolsHub, cfg: &DevToolsConsole) -> Response {
    let started = Instant::now();
    let body: RedisActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(ActionResponse::err(format!("invalid JSON: {e}"), 0.0), 400);
        }
    };

    let op_upper = body.op.to_ascii_uppercase();
    if DENYLIST.contains(&op_upper.as_str()) && !cfg.allow_dangerous {
        return json_response(
            ActionResponse::err(format!("op `{op_upper}` forbidden"), 0.0),
            403,
        );
    }

    let pool = match req.try_state::<RedisPool>() {
        Some(p) => p,
        None => {
            return json_response(ActionResponse::err("Redis plugin not installed", 0.0), 503);
        }
    };

    let audit = json!({
        "domain": "redis",
        "op": body.op,
        "key": body.key,
        "channel": body.channel,
    });

    match run_op(pool.as_ref(), cfg, body).await {
        Ok(result) => {
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            hub.emit(
                "devtools.action",
                json!({ "domain": "redis", "ok": true, "detail": audit }),
            );
            json_response(ActionResponse::ok(result, ms), 200)
        }
        Err(e) => {
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            hub.emit(
                "devtools.action",
                json!({ "domain": "redis", "ok": false, "error": e, "detail": audit }),
            );
            json_response(ActionResponse::err(e, ms), 400)
        }
    }
}

async fn run_op(
    pool: &RedisPool,
    cfg: &DevToolsConsole,
    body: RedisActionRequest,
) -> Result<Value, String> {
    if let Some(fake) = pool.fake() {
        return run_op_fake(fake.as_ref(), cfg, body).await;
    }

    if let Some(db) = body.db {
        cfg.set_redis_db(db);
    }
    let db = cfg.redis_db();
    let mut conn = pool.get().map_err(|e| e.0)?;
    redis::cmd("SELECT")
        .arg(db)
        .query_async::<()>(&mut conn)
        .await
        .map_err(|e| e.to_string())?;

    match body.op.to_ascii_lowercase().as_str() {
        "select" => {
            let db = body.db.unwrap_or(db);
            cfg.set_redis_db(db);
            Ok(json!({ "db": db }))
        }
        "get" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let val: Option<String> = conn.get(key).await.map_err(|e| e.to_string())?;
            Ok(json!({ "value": val }))
        }
        "set" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let value = body.value.unwrap_or_default();
            if value.len() > cfg.body_limit.min(MAX_BODY) {
                return Err("value too large".into());
            }
            if let Some(ttl) = body.ttl_secs {
                redis::cmd("SETEX")
                    .arg(&key)
                    .arg(ttl)
                    .arg(&value)
                    .query_async::<()>(&mut conn)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                conn.set::<_, _, ()>(key, value)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Ok(json!({ "ok": true }))
        }
        "del" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let n: i64 = conn.del(key).await.map_err(|e| e.to_string())?;
            Ok(json!({ "deleted": n }))
        }
        "ttl" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let ttl: i64 = conn.ttl(key).await.map_err(|e| e.to_string())?;
            Ok(json!({ "ttl": ttl }))
        }
        "type" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let ty: String = redis::cmd("TYPE")
                .arg(key)
                .query_async(&mut conn)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "type": ty }))
        }
        "scan" => {
            let pattern = body.pattern.unwrap_or_else(|| "*".into());
            let cursor = body.cursor.unwrap_or(0);
            let (next, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({ "cursor": next, "keys": keys }))
        }
        "publish" => {
            let channel = body.channel.ok_or_else(|| "channel required".to_string())?;
            let message = body.value.unwrap_or_default();
            let n = pool
                .publish(&channel, message.as_bytes())
                .await
                .map_err(|e| e.0)?;
            Ok(json!({ "subscribers": n }))
        }
        other => Err(format!("unknown op `{other}`")),
    }
}

async fn run_op_fake(
    fake: &sova_redis::FakeRedis,
    cfg: &DevToolsConsole,
    body: RedisActionRequest,
) -> Result<Value, String> {
    if let Some(db) = body.db {
        cfg.set_redis_db(db);
    }
    let db = cfg.redis_db();
    fake.select(db).map_err(|e| e.0)?;

    match body.op.to_ascii_lowercase().as_str() {
        "select" => {
            let db = body.db.unwrap_or(db);
            cfg.set_redis_db(db);
            Ok(json!({ "db": db }))
        }
        "get" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let val = fake.get(db, &key).map_err(|e| e.0)?;
            Ok(json!({ "value": val }))
        }
        "set" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let value = body.value.unwrap_or_default();
            if value.len() > cfg.body_limit.min(MAX_BODY) {
                return Err("value too large".into());
            }
            if let Some(ttl) = body.ttl_secs {
                fake.setex(db, &key, ttl, &value).map_err(|e| e.0)?;
            } else {
                fake.set(db, &key, &value).map_err(|e| e.0)?;
            }
            Ok(json!({ "ok": true }))
        }
        "del" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let n = fake.del(db, &key).map_err(|e| e.0)?;
            Ok(json!({ "deleted": n }))
        }
        "ttl" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let ttl = fake.ttl(db, &key).map_err(|e| e.0)?;
            Ok(json!({ "ttl": ttl }))
        }
        "type" => {
            let key = body.key.ok_or_else(|| "key required".to_string())?;
            let ty = fake.key_type(db, &key).map_err(|e| e.0)?;
            Ok(json!({ "type": ty }))
        }
        "scan" => {
            let pattern = body.pattern.unwrap_or_else(|| "*".into());
            let cursor = body.cursor.unwrap_or(0);
            let (next, keys) = fake.scan(db, cursor, &pattern, 100).map_err(|e| e.0)?;
            Ok(json!({ "cursor": next, "keys": keys }))
        }
        "publish" => {
            let channel = body.channel.ok_or_else(|| "channel required".to_string())?;
            let message = body.value.unwrap_or_default();
            let n = fake
                .publish(&channel, message.as_bytes())
                .map_err(|e| e.0)?;
            Ok(json!({ "subscribers": n }))
        }
        other => Err(format!("unknown op `{other}`")),
    }
}

pub async fn stream_subscribe(req: Request, hub: &DevToolsHub, cfg: &DevToolsConsole) -> Response {
    let channel = req
        .query("channel")
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string();
    if channel.is_empty() {
        return Response::text("missing channel query param").status(400);
    }

    let pool = match req.try_state::<RedisPool>() {
        Some(p) => p,
        None => return Response::text("Redis not installed").status(503),
    };

    let ch = SseChannel::new(64);
    let sse_ch = ch.clone();
    let hub = hub.clone();
    let pool = pool.as_ref().clone();
    let db = cfg.redis_db();
    let ch_name = channel.clone();

    tokio::spawn(async move {
        if pool.is_fake() {
            let Ok(mut sub) = pool.subscribe_fake(&ch_name).await else {
                ch.publish(SseEvent::data(r#"{"error":"subscribe failed"}"#).event("error"));
                return;
            };
            hub.emit(
                "devtools.action",
                json!({ "domain": "redis", "op": "subscribe", "channel": ch_name }),
            );
            while let Some(msg) = sub.next().await {
                let payload = msg.payload_str().unwrap_or("").to_string();
                let data = json!({
                    "channel": msg.channel,
                    "payload": payload,
                });
                ch.publish(SseEvent::data(data.to_string()).event("message"));
            }
            return;
        }

        let Ok(mut conn) = pool.get() else {
            ch.publish(SseEvent::data(r#"{"error":"connect failed"}"#).event("error"));
            return;
        };
        let _ = redis::cmd("SELECT")
            .arg(db)
            .query_async::<()>(&mut conn)
            .await;
        drop(conn);

        let Ok(mut sub) = pool.subscribe([&ch_name]).await else {
            ch.publish(SseEvent::data(r#"{"error":"subscribe failed"}"#).event("error"));
            return;
        };

        hub.emit(
            "devtools.action",
            json!({ "domain": "redis", "op": "subscribe", "channel": ch_name }),
        );

        while let Some(msg) = sub.next().await {
            let payload = msg.payload_str().unwrap_or("").to_string();
            let data = json!({
                "channel": msg.channel,
                "payload": payload,
            });
            ch.publish(SseEvent::data(data.to_string()).event("message"));
        }
    });

    sse_response(&req, &sse_ch, Duration::from_secs(15))
}
