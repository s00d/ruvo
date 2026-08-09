---
title: tasks-store
editLink: false
---

# `tasks-store`

**TaskStore trait + memory / file / sql / redis backends**

| | |
|--|--|
| Crate | [`sova-tasks-store`](https://docs.rs/sova-tasks-store/0.1.1) `0.1.1` |
| Plugin id | `tasks-store` |
| Category | Data |

## Install

```bash
cargo add sova --features tasks-store
```

## Features

| Feature | What you get |
|---------|-------------|
| `tasks-file` | File `TaskStore`. |
| `tasks-redis` | Redis `TaskStore` on `RedisPool`. |
| `tasks-sql` | SQL `TaskStore` on `DbPool`. |
| `tasks-store` | `TaskStore` backends crate. |

## Overview

**When:** persist task queue (memory / file / sql / redis).

**Does:**
- `TaskStore` backends for `tasks`
- Feature-gated drivers

### Example

```rust
app.install(Tasks::new().store(RedisTaskStore::from_pool(pool)));
```

## Quick start

Backends for [tasks](/plugins/tasks). Prefer constructing the store and passing it into `Tasks::new(...)`:

```rust
let store = Arc::new(sova::tasks::Memory::new());
// SQL (same pool as Db):
// let store = Arc::new(sova::tasks::Sql::from_db_pool(&pool));

app.install(Tasks::new(store).job(/* … */));
```

Facade features: `tasks-store`, `tasks-file`, `tasks-sql`, `tasks-redis` (usually via `tasks`).

## Related

[`tasks`](/plugins/tasks) · [`redis`](/plugins/redis) · [`db`](/plugins/db)
