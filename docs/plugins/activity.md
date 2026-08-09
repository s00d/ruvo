---
title: activity
editLink: false
---

# `activity`

**Audit / activity log (who changed what)**

| | |
|--|--|
| Crate | [`sova-activity`](https://docs.rs/sova-activity/0.1.2) `0.1.2` |
| Plugin id | `activity` |
| Category | Ops |

## Install

```bash
cargo add sova --features activity
```

## Features

| Feature | What you get |
|---------|-------------|
| `activity` | Audit / activity log table + mount. |

## Overview

**When:** audit log of who changed what (admin / Fortify).

**Does:**
- DB table via `ActivityMigrator`
- HTTP mount for browsing entries
- `req.log_activity(action, subject_type, id, meta)`
- `auth-activity` records Fortify mutations

### Example

```rust
app.install(Db::from_env().migrations::<ActivityMigrator>());
app.install(Activity::new().mount("/activity"));
req.log_activity("note.created", "note", id, json!({ "title": title })).await?;
```

## Quick start

Audit log on top of Db (often with Fortify). Use `ActivityMigrator` (or your app migrator that includes it):

```rust
app.install(Db::from_env().migrations::<ActivityMigrator>());
app.install(Activity::new().mount("/activity"));

// In a handler:
req.log_activity("note.created", "note", id, json!({ "title": title }))
    .await?;
```

Feature `auth-activity` records Fortify mutations automatically. See `examples/cabinet`.

## Related

[`auth`](/plugins/auth) · [`observability`](/plugins/observability)
