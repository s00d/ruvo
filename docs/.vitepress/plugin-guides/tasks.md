**When:** background jobs, priorities, cron / interval schedules.

**Does:**
- Dispatch jobs from handlers
- CLI: `tasks list` / `schedule` / `run NAME`
- Toml `[schedule.*]` overrides

### Example

```rust
app.install(Tasks::new().register(Ping));
// dispatch:
Ping.dispatch(&req).await?;
```

### Config

```toml
[schedule.ping]
every = "15s"
```
