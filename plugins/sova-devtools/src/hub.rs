//! Process-wide ring buffer + SSE fan-out.

use crate::collector::{LogLine, RequestMeta, RequestSnapshot, now_ms};
use serde::Serialize;
use serde_json::{json, Value};
use sova_sse::{SseChannel, SseEvent};
use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

static SEQ: AtomicU64 = AtomicU64::new(1);

pub fn next_id() -> String {
    format!("dt-{}", SEQ.fetch_add(1, Ordering::Relaxed))
}

#[derive(Clone, Debug, Serialize)]
pub struct CustomEvent {
    pub id: String,
    pub kind: String,
    pub payload: Value,
    pub ts_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MemorySample {
    pub ts_ms: u64,
    pub rss_bytes: Option<u64>,
}

struct HubInner {
    requests: VecDeque<RequestSnapshot>,
    by_id: HashMap<String, RequestSnapshot>,
    logs: VecDeque<LogLine>,
    custom: VecDeque<CustomEvent>,
    memory: VecDeque<MemorySample>,
    plugins: Vec<String>,
    profile: String,
    event_seq: u64,
    custom_cap: usize,
    memory_cap: usize,
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
                custom: VecDeque::new(),
                memory: VecDeque::new(),
                plugins: Vec::new(),
                profile: String::new(),
                event_seq: 0,
                custom_cap: 100,
                memory_cap: 120,
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

    fn next_eid(g: &mut HubInner) -> String {
        g.event_seq += 1;
        g.event_seq.to_string()
    }

    pub fn push_snapshot(&self, snap: RequestSnapshot) {
        let meta = RequestMeta::from(&snap);
        let mut g = self.inner.lock().unwrap();
        let eid = Self::next_eid(&mut g);
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
        let eid = Self::next_eid(&mut g);
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

    /// Emit a custom application/plugin event onto the DevTools SSE feed.
    pub fn emit(&self, kind: impl Into<String>, payload: Value) {
        let ev = CustomEvent {
            id: next_id(),
            kind: kind.into(),
            payload,
            ts_ms: now_ms(),
        };
        let mut g = self.inner.lock().unwrap();
        let eid = Self::next_eid(&mut g);
        let cap = g.custom_cap;
        g.custom.push_back(ev.clone());
        while g.custom.len() > cap {
            g.custom.pop_front();
        }
        drop(g);
        let data = serde_json::to_string(&json!({
            "type": "custom",
            "event": ev,
        }))
        .unwrap_or_else(|_| "{}".into());
        self.channel
            .publish(SseEvent::data(data).id(eid).event("custom"));
    }

    pub fn push_memory_sample(&self, rss_bytes: Option<u64>) {
        let sample = MemorySample {
            ts_ms: now_ms(),
            rss_bytes,
        };
        let mut g = self.inner.lock().unwrap();
        let eid = Self::next_eid(&mut g);
        let cap = g.memory_cap;
        g.memory.push_back(sample.clone());
        while g.memory.len() > cap {
            g.memory.pop_front();
        }
        drop(g);
        let data = serde_json::to_string(&json!({
            "type": "memory.sample",
            "sample": sample,
        }))
        .unwrap_or_else(|_| "{}".into());
        self.channel
            .publish(SseEvent::data(data).id(eid).event("memory.sample"));
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

    pub fn recent_custom(&self, limit: usize) -> Vec<CustomEvent> {
        let g = self.inner.lock().unwrap();
        g.custom.iter().rev().take(limit).cloned().collect()
    }

    pub fn recent_memory(&self, limit: usize) -> Vec<MemorySample> {
        let g = self.inner.lock().unwrap();
        g.memory.iter().rev().take(limit).cloned().collect()
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

/// Best-effort process RSS (Linux `/proc/self/status`).
pub fn process_rss_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let s = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
                return Some(kb.saturating_mul(1024));
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Background RSS sampler → SSE `memory.sample`.
pub fn spawn_memory_sampler(hub: DevToolsHub, interval: Duration) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            hub.push_memory_sample(process_rss_bytes());
        }
    });
}

/// Forward auth/mail EventBus events into [`DevToolsHub::emit`].
pub fn wire_event_bus(app: &mut sova_core::App, hub: DevToolsHub) {
    let bus = app.events();

    #[cfg(feature = "auth")]
    {
        let h = hub.clone();
        bus.listen::<sova_auth::UserRegistered, _>(move |e| {
            h.emit(
                "auth.user_registered",
                json!({ "user_id": e.user_id, "email": e.email }),
            );
        });
        let h = hub.clone();
        bus.listen::<sova_auth::UserLoggedIn, _>(move |e| {
            h.emit(
                "auth.user_logged_in",
                json!({ "user_id": e.user_id, "email": e.email }),
            );
        });
    }

    #[cfg(feature = "mail")]
    {
        let h = hub.clone();
        bus.listen::<sova_mail::MailSent, _>(move |e| {
            h.emit(
                "mail.sent",
                json!({ "to": e.to, "subject": e.subject }),
            );
        });
    }

    let _ = bus;
    let _ = hub;
}

fn compile_features() -> Vec<&'static str> {
    #[allow(clippy::vec_init_then_push)]
    {
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
}
