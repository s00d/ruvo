---
title: redis
editLink: false
---

# `redis`

**Shared Redis/Valkey connection for KvStore, tasks, cache, pub/sub, queues**

| | |
|--|--|
| Crate | [`sova-redis`](https://docs.rs/sova-redis/0.1.3) `0.1.3` |
| Plugin id | `redis` |
| Category | Data |

## Install

```bash
cargo add sova --features redis
```

## Features

| Feature | What you get |
|---------|-------------|
| `redis` | — |

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

### Config

```toml
[redis]
url = "redis://127.0.0.1:6379"
```

```bash
REDIS_URL=redis://127.0.0.1:6379   # wins over [redis] url when set
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

```toml
[redis]
url = "redis://127.0.0.1:6379"
```

`REDIS_URL` wins over `[redis] url` when set.

## Examples

- [`examples/misc/redis`](https://github.com/s00d/sova/tree/master/examples/misc/redis)

## Related

[`store`](/plugins/store) · [`session`](/plugins/session) · [`tasks-store`](/plugins/tasks-store)
