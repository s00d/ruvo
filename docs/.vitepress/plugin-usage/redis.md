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
