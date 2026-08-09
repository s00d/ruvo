**When:** persist the task queue (memory / file / sql / redis) for [tasks](/plugins/tasks).

**Does:**
- `TaskStore` trait + backends under `sova::tasks::*`
- Pass `Arc<dyn TaskStore>` into `Tasks::new(...)`
- SQL table `sova_tasks`; Redis / file drivers behind features

### Example

```rust
use std::sync::Arc;
use sova::Tasks;

// memory (dev / tests)
let store = Arc::new(sova::tasks::Memory::new());

// SQL (same pool as Db) — feature tasks-sql
// let store = Arc::new(sova::tasks::Sql::from_db_pool(&pool));

// Redis — feature tasks-redis
// let store = Arc::new(sova::tasks::Redis::from_redis_pool(&redis_pool));

// File — feature tasks-file
// let store = Arc::new(sova::tasks::File::open("data/tasks").await?);

app.install(Tasks::new(store).job(/* … */));
```

### Features

| Feature | Backend |
|---------|---------|
| `tasks-store` | crate + `Memory` |
| `tasks-file` | `File` |
| `tasks-sql` | `Sql` on `DbPool` |
| `tasks-redis` | `Redis` on `RedisPool` |

Usually enabled transitively with `tasks` + the driver you need.

### Notes
- There is no `Tasks::store(...)` — construct the backend, then `Tasks::new(arc)`.
- Install `Db` / `Redis` before building SQL/Redis stores.
