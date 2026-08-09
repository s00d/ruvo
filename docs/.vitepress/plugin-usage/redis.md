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
