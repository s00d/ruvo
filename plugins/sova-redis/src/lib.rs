//! Shared Redis / Valkey pool for Sova (`KvStore`, tasks, cache, pub/sub, list queues).
//!
//! ```ignore
//! app.install(Redis::from_env());
//! let pool = app.try_state::<RedisPool>().unwrap().as_ref().clone();
//! pool.publish("events", b"hello").await?;
//! let mut sub = pool.subscribe(["events"]).await?;
//! while let Some(msg) = sub.next().await {
//!     println!("{}: {:?}", msg.channel, msg.payload_str());
//! }
//! pool.enqueue("jobs", b"payload").await?;
//! let item = pool.dequeue("jobs").await?;
//! ```

mod error;
mod messaging;
mod plugin;
mod pool;

pub use error::RedisError;
pub use messaging::{RedisMessage, RedisSubscriber};
pub use plugin::Redis;
pub use pool::{RedisExt, RedisPool};
