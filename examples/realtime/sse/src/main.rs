//! Minimal Server-Sent Events example.

use futures_util::stream;
use ruvo::{App, Response, Result};
use std::convert::Infallible;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.get("/events", |_req| async {
        let ticks = stream::unfold(0u32, |n| async move {
            if n >= 5 {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            Some((Ok::<_, Infallible>(format!("tick {n}")), n + 1))
        });
        Response::sse(ticks)
    });
    tracing::info!("SSE on http://127.0.0.1:3000/events");
    app.listen(3000).await
}
