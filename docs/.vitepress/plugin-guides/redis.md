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
