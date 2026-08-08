Backends for [tasks](/plugins/tasks). Prefer constructing the store and passing it into `Tasks::new(...)`:

```rust
let store = Arc::new(sova::tasks::Memory::new());
// SQL (same pool as Db):
// let store = Arc::new(sova::tasks::Sql::from_db_pool(&pool));

app.install(Tasks::new(store).job(/* … */));
```

Facade features: `tasks-store`, `tasks-file`, `tasks-sql`, `tasks-redis` (usually via `tasks`).
