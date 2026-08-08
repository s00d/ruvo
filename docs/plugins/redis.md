---
title: redis
editLink: false
---

# `redis`

**Shared Redis/Valkey connection for KvStore, tasks, cache, pub/sub, queues** · crate `ruvo-redis` · id `redis`

```bash
cargo add ruvo --features redis
```

| Feature | What you get |
|---------|-------------|
| `redis` | Shared Redis/Valkey pool (`ruvo-redis`). |

Shared Redis / Valkey pool for Ruvo (`KvStore`, tasks, cache, pub/sub, list queues).

```rust
 app.install(Redis::from_env());
 let pool = app.try_state::<RedisPool>().unwrap().as_ref().clone();
 pool.publish("events", b"hello").await?;
 let mut sub = pool.subscribe(["events"]).await?;
 while let Some(msg) = sub.next().await {
     println!("{}: {:?}", msg.channel, msg.payload_str());
 }
 pool.enqueue("jobs", b"payload").await?;
 let item = pool.dequeue("jobs").await?;
 ```

## Usage

Shared Redis for store/session/tasks — install beside a preset:

```rust
let mut app = App::api().title("API").version("1.0").into_app();
app.install(Redis::from_env());
```

```bash
cargo run -p redis_demo
```
