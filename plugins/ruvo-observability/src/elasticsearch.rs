//! Ship tracing log events to Elasticsearch via `_bulk` (feature `elasticsearch`).

use ruvo_core::{set_log_event_hook, LogRecord};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Config for the Elasticsearch log sink.
#[derive(Debug, Clone)]
pub struct ElasticsearchLog {
    pub url: String,
    pub index: String,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub batch_size: usize,
    pub flush_interval: Duration,
    pub service: String,
}

impl ElasticsearchLog {
    /// From env: `ELASTICSEARCH_URL` (required), `ELASTICSEARCH_INDEX` (default `ruvo-logs`),
    /// `ELASTICSEARCH_API_KEY` or `ELASTICSEARCH_USERNAME`/`ELASTICSEARCH_PASSWORD`,
    /// `RUVO_SERVICE_NAME` / `OTEL_SERVICE_NAME` for `service` field.
    pub fn from_env() -> Result<Self, String> {
        let url = std::env::var("ELASTICSEARCH_URL")
            .map_err(|_| "ELASTICSEARCH_URL not set".to_string())?;
        if url.is_empty() {
            return Err("ELASTICSEARCH_URL empty".into());
        }
        let index =
            std::env::var("ELASTICSEARCH_INDEX").unwrap_or_else(|_| "ruvo-logs".into());
        let api_key = std::env::var("ELASTICSEARCH_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let username = std::env::var("ELASTICSEARCH_USERNAME")
            .ok()
            .filter(|s| !s.is_empty());
        let password = std::env::var("ELASTICSEARCH_PASSWORD").ok();
        let service = std::env::var("RUVO_SERVICE_NAME")
            .or_else(|_| std::env::var("OTEL_SERVICE_NAME"))
            .unwrap_or_else(|_| "ruvo".into());
        let batch_size = std::env::var("ELASTICSEARCH_BATCH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50)
            .max(1);
        let flush_ms = std::env::var("ELASTICSEARCH_FLUSH_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000u64)
            .max(100);
        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            index,
            api_key,
            username,
            password,
            batch_size,
            flush_interval: Duration::from_millis(flush_ms),
            service,
        })
    }

    /// Register the global log hook and spawn a background bulk flusher.
    ///
    /// Works with the existing `LogConfig` / `ensure_tracing` subscriber via
    /// [`ruvo_core::set_log_event_hook`].
    pub fn install(self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<Value>(self.batch_size.saturating_mul(4).max(64));
        let service = self.service.clone();
        let cfg = Arc::new(self);
        let cfg_worker = Arc::clone(&cfg);
        tokio::spawn(async move {
            flush_loop(cfg_worker, rx).await;
        });

        set_log_event_hook(Arc::new(move |rec: LogRecord| {
            let mut fields = serde_json::Map::new();
            for (k, v) in &rec.fields {
                fields.insert(k.clone(), Value::String(v.clone()));
            }
            let doc = json!({
                "@timestamp": chrono::Utc::now().to_rfc3339(),
                "level": rec.level,
                "target": rec.target,
                "message": rec.message,
                "fields": fields,
                "service": service,
            });
            let _ = tx.try_send(doc);
        }))
        .map_err(|_| "log event hook already set".to_string())?;
        Ok(())
    }
}

/// Install from `ELASTICSEARCH_URL` (and related env).
pub fn install_from_env() -> Result<(), String> {
    ElasticsearchLog::from_env()?.install()
}

async fn flush_loop(cfg: Arc<ElasticsearchLog>, mut rx: mpsc::Receiver<Value>) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "elasticsearch log sink: client build failed");
            return;
        }
    };
    let mut buf: Vec<Value> = Vec::with_capacity(cfg.batch_size);
    let mut interval = tokio::time::interval(cfg.flush_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased;
            maybe = rx.recv() => {
                match maybe {
                    Some(doc) => {
                        buf.push(doc);
                        if buf.len() >= cfg.batch_size {
                            flush_batch(&client, &cfg, &mut buf).await;
                        }
                    }
                    None => {
                        if !buf.is_empty() {
                            flush_batch(&client, &cfg, &mut buf).await;
                        }
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                if !buf.is_empty() {
                    flush_batch(&client, &cfg, &mut buf).await;
                }
            }
        }
    }
}

async fn flush_batch(client: &reqwest::Client, cfg: &ElasticsearchLog, buf: &mut Vec<Value>) {
    if buf.is_empty() {
        return;
    }
    let mut body = String::new();
    for doc in buf.drain(..) {
        body.push_str(&format!(
            "{}\n",
            json!({ "create": { "_index": cfg.index } })
        ));
        body.push_str(&format!("{doc}\n"));
    }
    let url = format!("{}/_bulk", cfg.url);
    let mut req = client
        .post(&url)
        .header("content-type", "application/x-ndjson")
        .body(body);
    if let Some(key) = &cfg.api_key {
        req = req.header("authorization", format!("ApiKey {key}"));
    } else if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) {
        req = req.basic_auth(u, Some(p));
    }
    match req.send().await {
        Ok(res) => {
            if !res.status().is_success() {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                tracing::warn!(%status, body = %text, "elasticsearch bulk failed");
            }
        }
        Err(e) => tracing::warn!(error = %e, "elasticsearch bulk request failed"),
    }
}
