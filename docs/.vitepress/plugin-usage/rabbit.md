**Audience:** apps that need raw AMQP (RabbitMQ): publish/consume, routing, ack/nack, DLQ.

Not a `TaskStore` backend — use `sova-tasks` for job queues.

Facade feature `rabbit` enables `sova-rabbit/lapin` for live brokers.

## Fake (tests)

`FakeBroker` auto-creates exchanges on publish (test convenience). Topic binding `#` is a test-only shortcut.

```rust
use sova::{Exchange, FakeBroker, QueueOpts, Rabbit, RabbitExt};
use bytes::Bytes;

let fake = FakeBroker::new();
app.install(Rabbit::fake(fake.clone()));

req.rabbit().declare_exchange(&Exchange::topic("events")).await?;
req.rabbit().declare_queue("jobs", &QueueOpts::durable()).await?;
req.rabbit().bind("jobs", "events", "user.*").await?;
req.rabbit().publish(&Exchange::topic("events"), "user.created", Bytes::from_static(b"{}")).await?;

let msg = fake.consume_one("jobs").await?.unwrap();
msg.ack().await?;
```

## Live

```rust
app.install(Rabbit::from_env().url("amqp://guest:guest@127.0.0.1:5672/%2f"));
```

Empty URL fails at startup (like Redis). Toml: `[rabbitmq] url=…` (or `[rabbit]`). Env: `AMQP_URL` / `RABBITMQ_URL`.

## Background consumer

For workers, prefer [`RabbitConsumer`] over polling `consume_one`:

```rust
use sova::{RabbitConsumer, RabbitExt};

RabbitConsumer::new("jobs", |msg| async move {
    // process msg.body
    msg.ack().await?;
    Ok(())
})
.prefetch(10)
.install(&mut app);
```

DLQ helper: `req.rabbit().setup_dlq("jobs", "dlx", "dlq", "jobs").await?`.

Example: [`examples/api/api_rabbit`](https://github.com/s00d/sova/tree/master/examples/api/api_rabbit).
