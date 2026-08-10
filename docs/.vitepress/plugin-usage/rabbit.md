**Audience:** apps that need raw AMQP (RabbitMQ): publish/consume, routing, ack/nack, DLQ.

Not a `TaskStore` backend — use `sova-tasks` for job queues.

## Fake (tests)

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

Toml: `[rabbitmq] url=…` (or `[rabbit]`). Env: `AMQP_URL` / `RABBITMQ_URL`.

DLQ helper: `req.rabbit().setup_dlq("jobs", "dlx", "dlq", "jobs").await?`.
