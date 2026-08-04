//! SSE channel helpers: fan-out, `Last-Event-ID`, keep-alive.

use futures_util::stream::{self, Stream, StreamExt};
use ruvo_core::{Request, Response};
use std::collections::VecDeque;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

/// In-process pub/sub for SSE clients.
#[derive(Clone)]
pub struct SseChannel {
    tx: broadcast::Sender<SseEvent>,
    history: Arc<Mutex<VecDeque<SseEvent>>>,
    history_cap: usize,
}

#[derive(Clone, Debug)]
pub struct SseEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: String,
}

impl SseEvent {
    pub fn data(data: impl Into<String>) -> Self {
        Self {
            id: None,
            event: None,
            data: data.into(),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn event(mut self, name: impl Into<String>) -> Self {
        self.event = Some(name.into());
        self
    }

    pub fn format(&self) -> String {
        let mut out = String::new();
        if let Some(id) = &self.id {
            out.push_str(&format!("id: {id}\n"));
        }
        if let Some(ev) = &self.event {
            out.push_str(&format!("event: {ev}\n"));
        }
        for line in self.data.lines() {
            out.push_str("data: ");
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
        out
    }
}

impl SseChannel {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(16));
        Self {
            tx,
            history: Arc::new(Mutex::new(VecDeque::new())),
            history_cap: 100,
        }
    }

    pub fn history_cap(mut self, n: usize) -> Self {
        self.history_cap = n.max(1);
        self
    }

    pub fn publish(&self, event: SseEvent) {
        if event.id.is_some() {
            let mut h = self.history.lock().unwrap();
            h.push_back(event.clone());
            while h.len() > self.history_cap {
                h.pop_front();
            }
        }
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SseEvent> {
        self.tx.subscribe()
    }

    /// Events after `last_id` from history (exclusive), then live stream.
    pub fn replay_after(&self, last_id: Option<&str>) -> Vec<SseEvent> {
        let h = self.history.lock().unwrap();
        let Some(last) = last_id else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut past = false;
        for e in h.iter() {
            if past {
                out.push(e.clone());
            } else if e.id.as_deref() == Some(last) {
                past = true;
            }
        }
        out
    }
}

/// Build an SSE [`Response`] with optional keep-alive comments.
pub fn sse_response(
    req: &Request,
    channel: &SseChannel,
    keep_alive: Duration,
) -> Response {
    let last_id = req
        .header("last-event-id")
        .map(|s| s.to_string());
    let replay = channel.replay_after(last_id.as_deref());
    let rx = channel.subscribe();

    let live = BroadcastStream::new(rx).filter_map(|r| async move {
        r.ok()
    });

    let replay_stream = stream::iter(replay);
    let events = replay_stream.chain(live);

    let keepalive = stream::unfold((), move |_| async move {
        tokio::time::sleep(keep_alive).await;
        Some((SseEvent::data(""), ()))
    })
    .map(|_e| ": keepalive\n\n".to_string());

    let data_stream = events.map(|e| {
        if e.data.is_empty() && e.id.is_none() && e.event.is_none() {
            ": keepalive\n\n".to_string()
        } else {
            e.format()
        }
    });

    let merged = stream::select(
        data_stream.map(Ok::<_, Infallible>),
        keepalive.map(Ok::<_, Infallible>),
    );

    Response::sse(merged)
}

/// Type alias for boxed SSE byte streams if needed by callers.
pub type SseStream = Pin<Box<dyn Stream<Item = Result<String, Infallible>> + Send>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_and_replay() {
        let ch = SseChannel::new(8);
        ch.publish(SseEvent::data("a").id("1"));
        ch.publish(SseEvent::data("b").id("2"));
        let replay = ch.replay_after(Some("1"));
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].data, "b");
        assert!(SseEvent::data("x").id("9").format().contains("id: 9"));
    }

    #[test]
    fn replay_skips_unknown_last_event_id() {
        let ch = SseChannel::new(8);
        ch.publish(SseEvent::data("a").id("1"));
        ch.publish(SseEvent::data("b").id("2"));
        assert!(ch.replay_after(Some("missing")).is_empty());
        assert!(ch.replay_after(None).is_empty());
    }

    #[test]
    fn replay_respects_history_cap() {
        let ch = SseChannel::new(8).history_cap(2);
        ch.publish(SseEvent::data("a").id("1"));
        ch.publish(SseEvent::data("b").id("2"));
        ch.publish(SseEvent::data("c").id("3"));
        assert!(ch.replay_after(Some("1")).is_empty(), "oldest id evicted");
        let replay = ch.replay_after(Some("2"));
        assert_eq!(replay.len(), 1);
        assert_eq!(replay[0].id.as_deref(), Some("3"));
    }

    #[tokio::test]
    async fn keepalive_comment_in_stream() {
        use http::Method;
        use http_body_util::BodyExt;
        use ruvo_core::extend::Body;

        let ch = SseChannel::new(8);
        let req = Request::new(Method::GET, "/events");
        let mut res = sse_response(&req, &ch, Duration::from_millis(30));
        let body = res.take_body();
        let Body::Stream(mut stream) = body else {
            panic!("expected sse stream body");
        };
        let frame = tokio::time::timeout(Duration::from_millis(200), stream.frame())
            .await
            .expect("timeout waiting for keepalive")
            .expect("stream ended")
            .expect("frame error");
        let chunk = frame.into_data().expect("data frame");
        let text = String::from_utf8_lossy(&chunk);
        assert!(text.contains("keepalive"), "expected comment keepalive, got {text:?}");
    }
}
