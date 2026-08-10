[![crates.io](https://img.shields.io/crates/v/sova-rabbit?style=for-the-badge)](https://crates.io/crates/sova-rabbit)
[![docs.rs](https://img.shields.io/docsrs/sova-rabbit?style=for-the-badge)](https://docs.rs/sova-rabbit)
[![License](https://img.shields.io/crates/l/sova-rabbit?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)

# sova-rabbit

Raw RabbitMQ / AMQP for Sova: publish / consume, exchange+routing, ack/nack, DLQ helpers, `FakeBroker`, `RabbitConsumer` worker.

**Guide:** [https://s00d.github.io/sova/plugins/rabbit](https://s00d.github.io/sova/plugins/rabbit)

## Features

| Feature | Description |
|---------|-------------|
| `lapin` (default) | Live AMQP via lapin |
| (none) | FakeBroker only — used when lapin disabled |

Facade `sova/rabbit` enables `lapin`.

## Install

```bash
cargo add sova --features rabbit
```

## License

MIT — see [LICENSE](LICENSE).
