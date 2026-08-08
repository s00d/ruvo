//! SseChannel: publish, subscribe, Last-Event-ID history, format.

use sova_sse::{SseChannel, SseEvent};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

#[test]
fn format_includes_id_event_data() {
    let formatted = SseEvent::data("line1\nline2")
        .id("42")
        .event("msg")
        .format();
    assert!(formatted.contains("id: 42\n"));
    assert!(formatted.contains("event: msg\n"));
    assert!(formatted.contains("data: line1\n"));
    assert!(formatted.contains("data: line2\n"));
    assert!(formatted.ends_with("\n\n"));
}

#[tokio::test]
async fn publish_subscribe() {
    let ch = SseChannel::new(16);
    let mut rx = ch.subscribe();
    ch.publish(SseEvent::data("hello").id("1").event("ping"));
    let ev = rx.recv().await.expect("event");
    assert_eq!(ev.data, "hello");
    assert_eq!(ev.id.as_deref(), Some("1"));
    assert_eq!(ev.event.as_deref(), Some("ping"));
}

#[test]
fn history_last_event_id() {
    let ch = SseChannel::new(8).history_cap(10);
    ch.publish(SseEvent::data("a").id("1"));
    ch.publish(SseEvent::data("b").id("2"));
    ch.publish(SseEvent::data("c").id("3"));

    let after_1 = ch.replay_after(Some("1"));
    assert_eq!(after_1.len(), 2);
    assert_eq!(after_1[0].data, "b");
    assert_eq!(after_1[1].data, "c");

    assert!(ch.replay_after(None).is_empty());
    assert!(ch.replay_after(Some("missing")).is_empty());
}

#[tokio::test]
async fn subscribe_receives_live_after_publish() {
    let ch = SseChannel::new(8);
    ch.publish(SseEvent::data("old").id("1"));
    assert!(ch.replay_after(Some("1")).is_empty());

    let mut rx = ch.subscribe();
    ch.publish(SseEvent::data("live").id("2"));
    match tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
        Ok(Ok(ev)) => {
            assert_eq!(ev.data, "live");
            assert_eq!(ev.id.as_deref(), Some("2"));
        }
        Ok(Err(RecvError::Lagged(_))) => panic!("lagged"),
        Ok(Err(RecvError::Closed)) => panic!("closed"),
        Err(_) => panic!("timeout"),
    }
}

#[tokio::test]
async fn sse_response_replays_after_last_event_id() {
    use http::Method;
    use http_body_util::BodyExt;
    use sova_core::extend::Body;
    use sova_core::Request;
    use sova_sse::sse_response;

    let ch = SseChannel::new(8);
    ch.publish(SseEvent::data("a").id("1"));
    ch.publish(SseEvent::data("b").id("2"));

    let req = Request::builder()
        .method(Method::GET)
        .path("/events")
        .header("last-event-id", "1")
        .build();
    let mut res = sse_response(&req, &ch, Duration::from_secs(60));
    let body = res.take_body();
    let Body::Stream(mut stream) = body else {
        panic!("expected sse stream");
    };
    let frame = tokio::time::timeout(Duration::from_millis(200), stream.frame())
        .await
        .expect("timeout")
        .expect("ended")
        .expect("ok");
    let chunk = frame.into_data().expect("data");
    let text = String::from_utf8_lossy(&chunk);
    assert!(text.contains("id: 2"), "got {text:?}");
    assert!(text.contains("data: b"), "got {text:?}");
}
