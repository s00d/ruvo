---
title: notifications
editLink: false
---

# `notifications`

**DB inbox, channels with ACL, optional WS/mail** · crate `ruvo-notifications` · id `notifications`

```bash
cargo add ruvo --features notifications,notifications-auth,notifications-mail,notifications-templates,notifications-ws
```

| Feature | What you get |
|---------|-------------|
| `notifications` | DB inbox + channels (`ruvo-notifications`). |
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

DB inbox (+ optional WS/mail) on a full app — cabinet is the reference:

```rust
let mut app = App::web()
    .site("App")
    .public_url("https://example.com")
    .into_app();

app.install(Db::from_env().migrations::<CabinetMigrator>());
app.install(Ws::new());
app.install(Notifications::new() /* feature flags: ws / mail / auth / templates */);
```

Features: `notifications-ws`, `notifications-mail`, `notifications-auth`, `notifications-templates`.
