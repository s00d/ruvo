---
title: notifications
editLink: false
---

# `notifications`

**DB inbox, channels with ACL, optional WS/mail** · crate `sova-notifications` `0.1.4` · id `notifications`

```bash
cargo add sova --features notifications,notifications-auth,notifications-mail,notifications-templates,notifications-ws
```

| Feature | What you get |
|---------|-------------|
| `notifications` | DB inbox + channels (`sova-notifications`). |
| `notifications-auth` | Role/permission audiences. |
| `notifications-mail` | Mail delivery channel. |
| `notifications-templates` | Unread helpers in templates. |
| `notifications-ws` | Push notifications over WebSocket. |

Database notifications with named channels, ACL, optional WS / mail.

```rust
 app.install(
   Notifications::new()
     .channel(Channel::new("orders").publish("notifications.orders.publish"))
     .mount("/notifications")
     .guard(Fortify::guard())
 );
 Notify::to(user_id).channel("orders").event("order.shipped").title("Shipped").send(&req).await?;
 ```

## Usage

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
