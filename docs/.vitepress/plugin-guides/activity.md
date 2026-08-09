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
