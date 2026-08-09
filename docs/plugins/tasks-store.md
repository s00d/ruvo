---
title: tasks-store
editLink: false
---

# `tasks-store`

**TaskStore trait + memory / file / sql / redis backends** · crate `sova-tasks-store` `0.1.1` · id `tasks-store`

```bash
cargo add sova --features tasks-file,tasks-redis,tasks-sql,tasks-store
```

| Feature | What you get |
|---------|-------------|
| `tasks-file` | File TaskStore. |
| `tasks-redis` | Redis TaskStore on `RedisPool`. |
| `tasks-sql` | SQL TaskStore on `DbPool`. |
| `tasks-store` | TaskStore backends crate. |

Task queue store for Sova.

 Trait is stable (memory + file + sql + redis backends).
 Queue claim/lease is **not** plain KvStore.

## Usage

Backends for [tasks](/plugins/tasks). Prefer constructing the store and passing it into `Tasks::new(...)`:

```rust
let store = Arc::new(sova::tasks::Memory::new());
// SQL (same pool as Db):
// let store = Arc::new(sova::tasks::Sql::from_db_pool(&pool));

app.install(Tasks::new(store).job(/* … */));
```

Facade features: `tasks-store`, `tasks-file`, `tasks-sql`, `tasks-redis` (usually via `tasks`).
