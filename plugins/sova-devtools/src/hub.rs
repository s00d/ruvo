//! Process-wide ring buffer + SSE fan-out.

use crate::collector::{LogLine, RequestMeta, RequestSnapshot};
use serde_json::json;
use sova_sse::{SseChannel, SseEvent};
use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

static SEQ: AtomicU64 = AtomicU64::new(1);

pub fn next_id() -> String {
    format!("dt-{}", SEQ.fetch_add(1, Ordering::Relaxed))
}

struct HubInner {
    requests: VecDeque<RequestSnapshot>,
    by_id: HashMap<String, RequestSnapshot>,
    logs: VecDeque<LogLine>,
    plugins: Vec<String>,
    profile: String,
    event_seq: u64,
}

/// Shared DevTools state installed on the app.
#[derive(Clone)]
pub struct DevToolsHub {
    inner: Arc<Mutex<HubInner>>,
    pub channel: SseChannel,
    request_cap: usize,
    log_cap: usize,
}

impl DevToolsHub {
    pub fn new(request_cap: usize, log_cap: usize) -> Self {
        let channel = SseChannel::new(256).history_cap(100);
        Self {
            inner: Arc::new(Mutex::new(HubInner {
                requests: VecDeque::new(),
                by_id: HashMap::new(),
                logs: VecDeque::new(),
                plugins: Vec::new(),
                profile: String::new(),
                event_seq: 0,
            })),
            channel,
            request_cap: request_cap.max(10),
            log_cap: log_cap.max(50),
        }
    }

    pub fn set_config_info(&self, plugins: Vec<String>, profile: String) {
        let mut g = self.inner.lock().unwrap();
        g.plugins = plugins;
        g.profile = profile;
    }

    pub fn push_snapshot(&self, snap: RequestSnapshot) {
        let meta = RequestMeta::from(&snap);
        let mut g = self.inner.lock().unwrap();
        g.event_seq += 1;
        let eid = g.event_seq.to_string();
        g.by_id.insert(snap.id.clone(), snap.clone());
        g.requests.push_back(snap);
        while g.requests.len() > self.request_cap {
            if let Some(old) = g.requests.pop_front() {
                g.by_id.remove(&old.id);
            }
        }
        drop(g);
        let data = serde_json::to_string(&json!({
            "type": "request.finished",
            "meta": meta,
        }))
        .unwrap_or_else(|_| "{}".into());
        self.channel.publish(
            SseEvent::data(data)
                .id(eid)
                .event("request.finished"),
        );
    }

    pub fn push_log(&self, line: LogLine) {
        let mut g = self.inner.lock().unwrap();
        g.event_seq += 1;
        let eid = g.event_seq.to_string();
        // Attach to open bag via request_id if middleware left one — also keep site feed.
        g.logs.push_back(line.clone());
        while g.logs.len() > self.log_cap {
            g.logs.pop_front();
        }
        drop(g);
        let data = serde_json::to_string(&json!({
            "type": "log.line",
            "line": line,
        }))
        .unwrap_or_else(|_| "{}".into());
        self.channel
            .publish(SseEvent::data(data).id(eid).event("log.line"));
    }

    pub fn get(&self, id: &str) -> Option<RequestSnapshot> {
        self.inner.lock().unwrap().by_id.get(id).cloned()
    }

    pub fn list_meta(&self, limit: usize) -> Vec<RequestMeta> {
        let g = self.inner.lock().unwrap();
        g.requests
            .iter()
            .rev()
            .take(limit)
            .map(RequestMeta::from)
            .collect()
    }

    pub fn recent_logs(&self, limit: usize) -> Vec<LogLine> {
        let g = self.inner.lock().unwrap();
        g.logs.iter().rev().take(limit).cloned().collect()
    }

    pub fn config_json(&self) -> serde_json::Value {
        let g = self.inner.lock().unwrap();
        json!({
            "profile": g.profile,
            "plugins": g.plugins,
            "features": compile_features(),
        })
    }
}

fn compile_features() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut v = Vec::new();
    #[cfg(feature = "session")]
    v.push("session");
    #[cfg(feature = "mail")]
    v.push("mail");
    #[cfg(feature = "http")]
    v.push("http");
    #[cfg(feature = "db")]
    v.push("db");
    #[cfg(feature = "tasks")]
    v.push("tasks");
    #[cfg(feature = "auth")]
    v.push("auth");
    #[cfg(feature = "i18n")]
    v.push("i18n");
    #[cfg(feature = "csrf")]
    v.push("csrf");
    #[cfg(feature = "passport")]
    v.push("passport");
    #[cfg(feature = "store")]
    v.push("store");
    #[cfg(feature = "redis")]
    v.push("redis");
    #[cfg(feature = "rate-limit")]
    v.push("rate-limit");
    v
}
