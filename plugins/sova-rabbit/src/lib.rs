//! RabbitMQ / AMQP for Sova — raw broker API + [`FakeBroker`].
//!
//! ```ignore
//! use sova_rabbit::{Exchange, FakeBroker, Rabbit, RabbitExt};
//!
//! let fake = FakeBroker::new();
//! app.install(Rabbit::fake(fake.clone()));
//! // req.rabbit().publish(Exchange::topic("events"), "user.created", b"{}").await?;
//! ```

mod broker;
mod consumer;
mod error;
mod fake;
mod plugin;
#[cfg(feature = "lapin")]
mod real;
mod trace;

pub use broker::{Broker, Delivery, Exchange, ExchangeKind, QueueOpts, SharedBroker};
pub use consumer::RabbitConsumer;
pub use error::RabbitError;
pub use fake::FakeBroker;
pub use plugin::{Rabbit, RabbitBound, RabbitExt};
