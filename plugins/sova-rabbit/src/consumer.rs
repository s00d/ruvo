//! Long-running queue consumer as [`BackgroundService`].

use crate::broker::{Delivery, SharedBroker};
use crate::error::RabbitError;
use sova_core::extend::{wait_shutdown, BoxFuture, StateMap};
use sova_core::{App, BackgroundService, Shutdown};
use std::sync::Arc;
use std::time::Duration;

type Handler = Arc<dyn Fn(Delivery) -> BoxFuture<Result<(), RabbitError>> + Send + Sync>;

/// Background AMQP consumer (`app.service(RabbitConsumer::new(...))`).
pub struct RabbitConsumer {
    queue: String,
    prefetch: u16,
    poll: Duration,
    handler: Handler,
}

impl RabbitConsumer {
    pub fn new<F, Fut>(queue: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Delivery) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), RabbitError>> + Send + 'static,
    {
        let handler: Handler = Arc::new(move |delivery| Box::pin(handler(delivery)));
        Self {
            queue: queue.into(),
            prefetch: 0,
            poll: Duration::from_millis(200),
            handler,
        }
    }

    pub fn prefetch(mut self, n: u16) -> Self {
        self.prefetch = n;
        self
    }

    pub fn poll_interval(mut self, d: Duration) -> Self {
        self.poll = d;
        self
    }

    pub fn install(self, app: &mut App) {
        app.service(RabbitConsumerService {
            queue: self.queue,
            prefetch: self.prefetch,
            poll: self.poll,
            handler: self.handler,
        });
    }
}

pub(crate) struct RabbitConsumerService {
    queue: String,
    prefetch: u16,
    poll: Duration,
    handler: Handler,
}

impl BackgroundService for RabbitConsumerService {
    fn name(&self) -> &str {
        "rabbit-consumer"
    }

    fn run(self: Box<Self>, state: Arc<StateMap>, shutdown: Shutdown) -> BoxFuture<()> {
        Box::pin(async move {
            let broker = match state.get::<SharedBroker>() {
                Some(b) => b,
                None => {
                    tracing::error!("rabbit consumer: SharedBroker not in app state");
                    return;
                }
            };
            if self.prefetch > 0 {
                tracing::debug!(
                    queue = %self.queue,
                    prefetch = self.prefetch,
                    "rabbit consumer prefetch (lapin QoS applied on connect when supported)"
                );
            }
            loop {
                if shutdown.is_triggered() {
                    break;
                }
                tokio::select! {
                    _ = wait_shutdown(shutdown.clone()) => break,
                    _ = async {
                        match broker.consume_one(&self.queue).await {
                            Ok(Some(delivery)) => {
                                if let Err(e) = (self.handler)(delivery).await {
                                    tracing::warn!(queue = %self.queue, error = %e, "rabbit consumer handler failed");
                                }
                            }
                            Ok(None) => {
                                tokio::time::sleep(self.poll).await;
                            }
                            Err(e) => {
                                tracing::warn!(queue = %self.queue, error = %e, "rabbit consume failed");
                                tokio::time::sleep(self.poll).await;
                            }
                        }
                    } => {}
                }
            }
        })
    }
}
