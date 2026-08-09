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
