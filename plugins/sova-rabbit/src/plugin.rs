//! Plugin + `req.rabbit()`.

use crate::broker::{Broker, Exchange, QueueOpts, SharedBroker};
use crate::error::RabbitError;
use crate::fake::FakeBroker;
use bytes::Bytes;
use serde_json::json;
use sova_core::{App, DevToolsConfigRegistry, Error, Plugin, Request};
use std::sync::Arc;

#[cfg(feature = "lapin")]
use crate::real::LapinBroker;

pub struct Rabbit {
    mode: Mode,
}

enum Mode {
    Url(String),
    Fake(FakeBroker),
}

impl Rabbit {
    pub fn from_env() -> Self {
        Self {
            mode: Mode::Url(
                std::env::var("AMQP_URL")
                    .or_else(|_| std::env::var("RABBITMQ_URL"))
                    .unwrap_or_default(),
            ),
        }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.mode = Mode::Url(url.into());
        self
    }

    pub fn fake(fake: FakeBroker) -> Self {
        Self {
            mode: Mode::Fake(fake),
        }
    }
}

impl Plugin for Rabbit {
    fn id(&self) -> &'static str {
        "rabbit"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("RabbitMQ")
            .description("Raw AMQP broker (publish/consume, DLQ, FakeBroker)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("rabbitmq").or_else(|| doc.section("rabbit")) {
                if let Mode::Url(url) = &mut self.mode {
                    if url.is_empty() {
                        if let Some(u) = section.get("url").and_then(|v| v.as_str()) {
                            *url = u.to_string();
                        }
                    }
                }
            }
        }

        match self.mode {
            Mode::Fake(fake) => {
                let broker: SharedBroker = Arc::new(fake);
                app.state(broker);
                register_devtools_mount(app, "fake");
            }
            Mode::Url(url) => {
                if url.is_empty() {
                    app.on_startup(|_state| async {
                        Err(Error::Internal(
                            "rabbit url is empty; set AMQP_URL / RABBITMQ_URL or [rabbitmq] url in sova.toml"
                                .into(),
                        ))
                    });
                    return;
                }
                register_devtools_mount(app, "live");
                #[cfg(feature = "lapin")]
                {
                    let broker = LapinBroker::new(url);
                    let shared: SharedBroker = Arc::new(broker.clone());
                    app.state(shared);
                    let b = broker.clone();
                    app.on_startup(move |_state| {
                        let b = b.clone();
                        async move { b.connect().await.map_err(Into::into) }
                    });
                }
                #[cfg(not(feature = "lapin"))]
                {
                    let _ = url;
                    app.on_startup(|_state| async {
                        Err(Error::Internal(
                            "sova-rabbit built without `lapin` feature; enable `sova/rabbit` or `sova-rabbit/lapin`"
                                .into(),
                        ))
                    });
                }
            }
        }
    }
}

fn register_devtools_mount(app: &mut App, mode: &str) {
    if app.try_state::<DevToolsConfigRegistry>().is_none() {
        app.state(DevToolsConfigRegistry::default());
    }
    let reg = app
        .try_state::<DevToolsConfigRegistry>()
        .expect("DevToolsConfigRegistry");
    reg.set("rabbit", json!({ "mode": mode }));
}

pub trait RabbitExt {
    fn rabbit(&self) -> RabbitBound;
    fn try_rabbit(&self) -> Option<RabbitBound>;
}

impl RabbitExt for Request {
    fn rabbit(&self) -> RabbitBound {
        RabbitBound {
            broker: self.state::<SharedBroker>(),
        }
    }

    fn try_rabbit(&self) -> Option<RabbitBound> {
        self.try_state::<SharedBroker>()
            .map(|broker| RabbitBound { broker })
    }
}

pub struct RabbitBound {
    broker: Arc<SharedBroker>,
}

impl RabbitBound {
    pub fn broker(&self) -> &dyn Broker {
        self.broker.as_ref().as_ref()
    }

    pub async fn declare_exchange(&self, exchange: &Exchange) -> Result<(), RabbitError> {
        self.broker.declare_exchange(exchange).await
    }

    pub async fn declare_queue(&self, name: &str, opts: &QueueOpts) -> Result<(), RabbitError> {
        self.broker.declare_queue(name, opts).await
    }

    pub async fn bind(
        &self,
        queue: &str,
        exchange: &str,
        routing_key: &str,
    ) -> Result<(), RabbitError> {
        self.broker.bind(queue, exchange, routing_key).await
    }

    pub async fn publish(
        &self,
        exchange: &Exchange,
        routing_key: &str,
        body: impl Into<Bytes>,
    ) -> Result<(), RabbitError> {
        self.broker
            .publish(exchange, routing_key, body.into())
            .await
    }

    /// Poll one message (tests / scripts). For workers prefer [`RabbitConsumer`].
    pub async fn consume_one(&self, queue: &str) -> Result<Option<crate::Delivery>, RabbitError> {
        self.broker.consume_one(queue).await
    }

    /// Convenience: declare queue + DLX exchange + bind DLQ.
    pub async fn setup_dlq(
        &self,
        queue: &str,
        dlx: &str,
        dlq: &str,
        routing_key: &str,
    ) -> Result<(), RabbitError> {
        let ex = Exchange::direct(dlx);
        self.declare_exchange(&ex).await?;
        self.declare_queue(dlq, &QueueOpts::durable()).await?;
        self.bind(dlq, dlx, routing_key).await?;
        self.declare_queue(queue, &QueueOpts::durable().with_dlq(dlx, routing_key))
            .await?;
        Ok(())
    }
}
