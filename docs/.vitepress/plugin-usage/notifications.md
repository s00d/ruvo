DB inbox (+ optional WS / mail). Install Db first; install `Ws` before `.ws_path(...)`:

```rust
use sova::{Db, Notifications, NotificationsMigrator, Notify, Ws};

app.install(Db::from_env().migrations::<NotificationsMigrator>());
app.install(Ws::new());
app.install(
    Notifications::new()
        .channel(/* Channel::new("orders")… */)
        .mount("/notifications")
        .ws_path("/ws/notifications") // needs notifications-ws
        .guard(/* Fortify::guard() */),
);

// from a handler:
Notify::to(user_id)
    .channel("orders")
    .event("order.shipped")
    .title("Shipped")
    .send(&req)
    .await?;
```

Features: `notifications-ws`, `notifications-mail`, `notifications-auth`, `notifications-templates`.
