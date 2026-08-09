In-process typed events for plugins and apps.

```rust
use sova::{Event, EventBus};

struct UserRegistered { id: i64 }
impl Event for UserRegistered {
    fn name(&self) -> &'static str { "user.registered" }
}

let bus = app.events(); // inserts EventBus into app.state on first call
bus.listen::<UserRegistered, _>(|e| {
    tracing::info!(user_id = e.id, "registered");
    // optional: tokio::spawn / TaskBackend::dispatch — not in core
});
```

Listeners run **synchronously** in registration order inside `dispatch`. For async work, spawn from the listener. See also [Extractors & Problem+](./extractors).
