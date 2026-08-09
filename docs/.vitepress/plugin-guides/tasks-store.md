**When:** persist the task queue (memory / file / sql / redis) for [tasks](/plugins/tasks).

**Does:**
- `TaskStore` trait + backends under `sova::tasks::*`
- Soft-wire: `Tasks::memory()` / `Tasks::sql(&app)` / `Tasks::redis(&app)`
- Or `Tasks::new(Arc<dyn TaskStore>)` for custom stores
- SQL table `sova_tasks`; Redis / file drivers behind features

### Example

```rust
app.install(Db::from_env());
app.install(Tasks::sql(&app).job(/* … */));

// app.install(Tasks::memory().job(/* … */));
// app.install(Redis::from_env());
// app.install(Tasks::redis(&app).job(/* … */));
```

### Features

| Feature | Backend |
|---------|---------|
| `tasks-store` | crate + `Memory` |
| `tasks-file` | `File` |
| `tasks-sql` | `Sql` on `DbPool` (+ `Tasks::sql`) |
| `tasks-redis` | `Redis` on `RedisPool` (+ `Tasks::redis`) |

Usually enabled transitively with `tasks` + the driver you need.
