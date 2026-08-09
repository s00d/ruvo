Audit log on top of Db (often with Fortify). Use `ActivityMigrator` (or your app migrator that includes it):

```rust
app.install(Db::from_env().migrations::<ActivityMigrator>());
app.install(Activity::new().mount("/activity"));

// In a handler:
req.log_activity("note.created", "note", id, json!({ "title": title }))
    .await?;
```

Feature `auth-activity` records Fortify mutations automatically. See [`examples/cabinet`](https://github.com/s00d/sova/tree/master/examples/cabinet).
