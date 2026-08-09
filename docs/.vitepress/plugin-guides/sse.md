**When:** Server-Sent Events streams to browsers.

**Does:**
- `SseChannel` + `SseEvent` (id / event / data)
- `sse_response` with keep-alive + Last-Event-ID replay
- No separate `Sse` plugin — `app.state(channel)`

### Example

```rust
let channel = SseChannel::new(64);
app.state(channel.clone());
app.get("/events", |req: Request| async move {
    let ch = req.state::<SseChannel>();
    sse_response(&req, &ch, Duration::from_secs(15))
});
```
