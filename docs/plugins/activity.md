---
title: activity
editLink: false
---

# `activity`

**Audit / activity log (who changed what)** · crate `sova-activity` `0.1.2` · id `activity`

```bash
cargo add sova --features activity
```

| Feature | What you get |
|---------|-------------|
| `activity` | Audit / activity log table (`sova-activity`). |

Laravel-style activity / audit log for Sova.

```rust
 app.install(Db::from_env().migrations::<ActivityMigrator>());
 // or compose: AuthMigrator::migrations() + ActivityMigrator::migrations()
 app.install(Activity::new().mount("/activity"));
 req.log_activity("note.created", "note", id, json!({})).await;
 ```

## Usage

Audit log on top of Db (often with Fortify). Use `ActivityMigrator` (or your app migrator that includes it):

```rust
app.install(Db::from_env().migrations::<ActivityMigrator>());
app.install(Activity::new().mount("/activity"));

// In a handler:
req.log_activity("note.created", "note", id, json!({ "title": title }))
    .await?;
```

Feature `auth-activity` records Fortify mutations automatically. See `examples/cabinet`.
