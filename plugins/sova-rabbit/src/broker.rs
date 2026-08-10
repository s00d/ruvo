//! Broker trait + message types.

use crate::error::RabbitError;
use async_trait::async_trait;
use bytes::Bytes;
use sova_core::extend::BoxFuture;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeKind {
    Direct,
    Topic,
    Fanout,
    Headers,
}

#[derive(Debug, Clone)]
pub struct Exchange {
    pub name: String,
    pub kind: ExchangeKind,
    pub durable: bool,
}

impl Exchange {
    pub fn direct(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: ExchangeKind::Direct,
            durable: true,
        }
    }

    pub fn topic(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: ExchangeKind::Topic,
            durable: true,
        }
    }

    pub fn fanout(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: ExchangeKind::Fanout,
            durable: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueueOpts {
    pub durable: bool,
    pub dead_letter_exchange: Option<String>,
    pub dead_letter_routing_key: Option<String>,
}

impl QueueOpts {
    pub fn durable() -> Self {
        Self {
            durable: true,
            ..Default::default()
        }
    }

    /// Declare with DLX (dead-letter exchange) + optional routing key.
    pub fn with_dlq(mut self, dlx: impl Into<String>, routing_key: impl Into<String>) -> Self {
        self.dead_letter_exchange = Some(dlx.into());
        self.dead_letter_routing_key = Some(routing_key.into());
        self
    }
}

/// Delivered message handle (ack / nack / reject).
pub struct Delivery {
    pub exchange: String,
    pub routing_key: String,
    pub body: Bytes,
    ack_fn: Box<dyn FnOnce(bool) -> BoxFuture<Result<(), RabbitError>> + Send>,
    nack_fn: Box<dyn FnOnce(bool) -> BoxFuture<Result<(), RabbitError>> + Send>,
}

impl Delivery {
    pub fn new<A, N, FA, FN>(
        exchange: impl Into<String>,
        routing_key: impl Into<String>,
        body: impl Into<Bytes>,
        ack: A,
        nack: N,
    ) -> Self
    where
        A: FnOnce(bool) -> FA + Send + 'static,
        FA: std::future::Future<Output = Result<(), RabbitError>> + Send + 'static,
        N: FnOnce(bool) -> FN + Send + 'static,
        FN: std::future::Future<Output = Result<(), RabbitError>> + Send + 'static,
    {
        Self {
            exchange: exchange.into(),
            routing_key: routing_key.into(),
            body: body.into(),
            ack_fn: Box::new(move |multiple| Box::pin(ack(multiple))),
            nack_fn: Box::new(move |requeue| Box::pin(nack(requeue))),
        }
    }

    pub async fn ack(self) -> Result<(), RabbitError> {
        (self.ack_fn)(false).await
    }

    pub async fn nack(self, requeue: bool) -> Result<(), RabbitError> {
        (self.nack_fn)(requeue).await
    }

    pub async fn reject(self, requeue: bool) -> Result<(), RabbitError> {
        self.nack(requeue).await
    }
}

#[async_trait]
pub trait Broker: Send + Sync {
    async fn declare_exchange(&self, exchange: &Exchange) -> Result<(), RabbitError>;
    async fn declare_queue(&self, name: &str, opts: &QueueOpts) -> Result<(), RabbitError>;
    async fn bind(
        &self,
        queue: &str,
        exchange: &str,
        routing_key: &str,
    ) -> Result<(), RabbitError>;

    async fn publish(
        &self,
        exchange: &Exchange,
        routing_key: &str,
        body: Bytes,
    ) -> Result<(), RabbitError>;

    /// Blocking-ish consume of next message (tests / simple workers).
    async fn consume_one(&self, queue: &str) -> Result<Option<Delivery>, RabbitError>;
}

pub type SharedBroker = Arc<dyn Broker>;
