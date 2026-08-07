//! SSE feed with channel + Last-Event-ID + keep-alive.

use ruvo::{sse_response, App, Request, Result, SseChannel, SseEvent};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let channel = SseChannel::new(64);
    let pub_ch = channel.clone();
    tokio::spawn(async move {
        let mut n = 0u64;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            n += 1;
            pub_ch.publish(SseEvent::data(format!("tick {n}")).id(n.to_string()));
        }
    });

    let mut app = App::new();
    app.state(channel);
    app.get("/events", |req: Request| async move {
        let ch = req.state::<SseChannel>();
        sse_response(&req, &ch, Duration::from_secs(15))
    });
    app.get("/", |_| async {
        ruvo::Response::html(
            r#"<!doctype html><pre id=o></pre>
<script>
const o=document.getElementById('o');
const es=new EventSource('/events');
es.onmessage=e=>{o.textContent+=e.data+'\n'};
</script>"#,
        )
    });
    tracing::info!("SSE feed http://127.0.0.1:3012/events");
    app.listen(3012).await
}
