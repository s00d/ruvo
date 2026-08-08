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
