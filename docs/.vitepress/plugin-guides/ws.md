**When:** WebSocket hubs (chat, live feeds).

**Does:**
- `app.install(Ws::new())` + `app.ws("/ws", handler)`
- Rooms hub, origin allowlist, max message size
- `session.join` / `hub().broadcast`

### Example

```rust
app.install(Ws::new().origins(["https://example.com"]));
app.ws("/ws", |mut session| async move {
    let _room = session.join("chat");
    while let Some(Ok(msg)) = session.recv().await { /* … */ }
});
```
