---
title: redis
editLink: false
---

# `redis`

**Shared Redis/Valkey connection for KvStore, tasks, cache, pub/sub, queues**

| | |
|--|--|
| Crate | [`sova-redis`](https://docs.rs/sova-redis/0.1.1) `0.1.1` |
| Plugin id | `redis` |
| Category | Data |

## Install

```bash
cargo add sova --features redis
```

## Features

| Feature | What you get |
|---------|-------------|
| `redis` | Shared Redis/Valkey pool (cache, pub/sub, queues). |

## Overview

**When:** shared Redis/Valkey for cache, sessions, tasks, pub/sub, queues.

**Does:**
- `Redis::from_env()` → `RedisPool` in state
- publish / subscribe / enqueue / dequeue
- Backs `store-redis`, `session-redis`, `tasks-redis`

### Example

```rust
app.install(Redis::from_env());
let pool = req.redis();
pool.publish("events", b"hello").await?;
```

## Quick start

Shared Redis/Valkey pool — install once, reuse for store / session / tasks:

```rust
use sova::{App, Redis, RedisExt};

app.install(Redis::from_env());

app.get("/pub", |req| async move {
    req.redis().publish("events", b"hello").await?;
    Ok("ok")
});
```

Env: `REDIS_URL` (or Valkey-compatible). Features that consume the pool: `store-redis`, `session-redis`, `tasks-redis`.

## Examples

- `examples/misc/redis`

## Related

[`store`](/plugins/store) · [`session`](/plugins/session) · [`tasks-store`](/plugins/tasks-store)
