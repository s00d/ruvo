---
title: notifications
editLink: false
---

# `notifications`

**DB inbox, channels with ACL, optional WS/mail**

| | |
|--|--|
| Crate | [`sova-notifications`](https://docs.rs/sova-notifications/0.1.5) `0.1.5` |
| Plugin id | `notifications` |
| Category | Realtime |

## Install

```bash
cargo add sova --features notifications
```

## Features

| Feature | What you get |
|---------|-------------|
| `notifications` | DB inbox + named channels / ACL. |
| `notifications-auth` | Role/permission audiences. |
| `notifications-mail` | Mail delivery channel. |
| `notifications-templates` | Unread helpers in templates. |
| `notifications-ws` | Push notifications over WebSocket. |

## Overview

**When:** in-app notification inbox (DB) with channels / ACL; optional WS + mail.

**Does:**
- Named channels + publish ACL
- HTTP mount for inbox
- `Notify::to(user).channel(…).send(&req)`
- Optional realtime + mail features

### Example

```rust
app.install(Db::from_env().migrations::<NotificationsMigrator>());
app.install(Notifications::new().mount("/notifications"));
Notify::to(user_id).channel("orders").title("Shipped").send(&req).await?;
```

## Quick start

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

## Examples

- [`examples/cabinet`](https://github.com/s00d/sova/tree/master/examples/cabinet)

## Related

[`db`](/plugins/db) · [`ws`](/plugins/ws) · [`mail`](/plugins/mail) · [`auth`](/plugins/auth)
