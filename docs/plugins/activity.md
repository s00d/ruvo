---
title: activity
editLink: false
---

# `activity`

**Audit / activity log (who changed what)** · crate `ruvo-activity` · id `activity`

```bash
cargo add ruvo --features activity
```

| Feature | What you get |
|---------|-------------|
| `activity` | Audit / activity log table (`ruvo-activity`). |

Laravel-style activity / audit log for Ruvo.

```rust
 app.install(Db::from_env().migrations::<ActivityMigrator>());
 // or compose: AuthMigrator::migrations() + ActivityMigrator::migrations()
 app.install(Activity::new().mount("/activity"));
 req.log_activity("note.created", "note", id, json!({})).await;
 ```

## Usage

Audit log on top of Db (often with Fortify). Compose migrators, then install the plugin:

```rust
let mut app = App::web()
    .site("App")
    .public_url("https://example.com")
    .into_app();

app.install(Db::from_env().migrations::<CabinetMigrator>()); // includes ActivityMigrator
app.install(Activity::new().mount("/activity"));

// In a handler:
req.log_activity("note.created", "note", id, json!({ "title": title }))
    .await?;
```

Feature `auth-activity` records Fortify mutations automatically. See cabinet.
