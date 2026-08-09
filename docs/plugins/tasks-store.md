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

## Quick start

Construct a backend, pass into `Tasks` (soft-wire helpers prefer pool from state):

```rust
use sova::{Db, Tasks};

app.install(Db::from_env());
app.install(Tasks::sql(&app).queues(["default"]).job(/* … */));

// or:
// app.install(Redis::from_env());
// app.install(Tasks::redis(&app).job(/* … */));
// app.install(Tasks::memory().job(/* … */));
```

Advanced (custom store):

```rust
use std::sync::Arc;
let pool = app.try_state::<sova::DbPool>().unwrap().as_ref().clone();
app.install(Tasks::new(Arc::new(sova::tasks::Sql::from_db_pool(&pool))).job(/* … */));
```

Features: `tasks-store`, `tasks-file`, `tasks-sql`, `tasks-redis`.

## Related

[`tasks`](/plugins/tasks) · [`redis`](/plugins/redis) · [`db`](/plugins/db)
