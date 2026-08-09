```rust
use sova::{sse_response, App, Request, Result, SseChannel, SseEvent};
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
    app.run().await
}
```

There is no `Sse` plugin — use `SseChannel` + `app.state` (see [`examples/realtime/sse_feed`](https://github.com/s00d/sova/tree/master/examples/realtime/sse_feed)).

```bash
cargo run -p sse_feed
```
