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

## Quick start

Construct a backend, pass into `Tasks::new`:

```rust
use std::sync::Arc;
use sova::{Db, Redis, Tasks};

app.install(Db::from_env());
app.install(Redis::from_env());

let pool = app.try_state::<sova::DbPool>().unwrap().as_ref().clone();
// let rpool = app.try_state::<sova::RedisPool>().unwrap().as_ref().clone();

let store = Arc::new(sova::tasks::Sql::from_db_pool(&pool));
// let store = Arc::new(sova::tasks::Redis::from_redis_pool(&rpool));
// let store = Arc::new(sova::tasks::Memory::new());
// let store = Arc::new(sova::tasks::File::open("data/tasks").await?);

app.install(Tasks::new(store).queues(["default"]).job(/* … */));
```

Features: `tasks-store`, `tasks-file`, `tasks-sql`, `tasks-redis`.

## Related

[`tasks`](/plugins/tasks) · [`redis`](/plugins/redis) · [`db`](/plugins/db)
