---
title: rabbit
editLink: false
---

# `rabbit`

**Raw AMQP broker (publish/consume, DLQ, FakeBroker)**

| | |
|--|--|
| Crate | [`sova-rabbit`](https://docs.rs/sova-rabbit/0.1.0) `0.1.0` |
| Plugin id | `rabbit` |
| Category | Integrations |

## Install

```bash
cargo add sova --features rabbit
```

## Features

| Feature | What you get |
|---------|-------------|
| `rabbit` | RabbitMQ / AMQP broker (`req.rabbit()`, FakeBroker). |

## Overview

RabbitMQ / AMQP for Sova — raw broker API + [`FakeBroker`].

```rust
 use sova_rabbit::{Exchange, FakeBroker, Rabbit, RabbitExt};

 let fake = FakeBroker::new();
 app.install(Rabbit::fake(fake.clone()));
 // req.rabbit().publish(Exchange::topic("events"), "user.created", b"{}").await?;
 ```

## Quick start

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

## Related

[`redis`](/plugins/redis) · [`tasks`](/plugins/tasks)
