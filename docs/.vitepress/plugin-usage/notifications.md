DB inbox (+ optional WS/mail). Install Db first; add `Ws` before `.ws_path(...)`:

```rust
app.install(Db::from_env().migrations::<NotificationsMigrator>());
app.install(Ws::new());
app.install(
    Notifications::new()
        .mount("/notifications")
        .ws_path("/ws/notifications"), // requires feature notifications-ws + Ws plugin
);
```

Features: `notifications-ws`, `notifications-mail`, `notifications-auth`, `notifications-templates`. Template helpers require installed Templates.
