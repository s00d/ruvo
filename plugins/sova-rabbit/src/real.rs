//! Lapin-backed broker (feature `lapin`).

use crate::broker::{Broker, Delivery, Exchange, ExchangeKind, QueueOpts};
use crate::error::RabbitError;
use async_trait::async_trait;
use bytes::Bytes;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions,
    ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind as LapinKind};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct LapinBroker {
    url: String,
    channel: Arc<RwLock<Option<Channel>>>,
}

impl LapinBroker {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            channel: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn connect(&self) -> Result<(), RabbitError> {
        let conn = Connection::connect(&self.url, ConnectionProperties::default())
            .await
            .map_err(|e| RabbitError::Msg(format!("connect: {e}")))?;
        let ch = conn
            .create_channel()
            .await
            .map_err(|e| RabbitError::Msg(format!("channel: {e}")))?;
        *self.channel.write().await = Some(ch);
        Ok(())
    }

    async fn ch(&self) -> Result<Channel, RabbitError> {
        self.channel
            .read()
            .await
            .clone()
            .ok_or(RabbitError::NotConnected)
    }
}

fn to_lapin_kind(k: ExchangeKind) -> LapinKind {
    match k {
        ExchangeKind::Direct => LapinKind::Direct,
        ExchangeKind::Topic => LapinKind::Topic,
        ExchangeKind::Fanout => LapinKind::Fanout,
        ExchangeKind::Headers => LapinKind::Headers,
    }
}

#[async_trait]
impl Broker for LapinBroker {
    async fn declare_exchange(&self, exchange: &Exchange) -> Result<(), RabbitError> {
        let ch = self.ch().await?;
        ch.exchange_declare(
            &exchange.name,
            to_lapin_kind(exchange.kind),
            ExchangeDeclareOptions {
                durable: exchange.durable,
                ..ExchangeDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(|e| RabbitError::Msg(e.to_string()))?;
        Ok(())
    }

    async fn declare_queue(&self, name: &str, opts: &QueueOpts) -> Result<(), RabbitError> {
        let ch = self.ch().await?;
        let mut args = FieldTable::default();
        if let Some(dlx) = &opts.dead_letter_exchange {
            args.insert(
                "x-dead-letter-exchange".into(),
                lapin::types::AMQPValue::LongString(dlx.clone().into()),
            );
        }
        if let Some(rk) = &opts.dead_letter_routing_key {
            args.insert(
                "x-dead-letter-routing-key".into(),
                lapin::types::AMQPValue::LongString(rk.clone().into()),
            );
        }
        ch.queue_declare(
            name,
            QueueDeclareOptions {
                durable: opts.durable,
                ..QueueDeclareOptions::default()
            },
            args,
        )
        .await
        .map_err(|e| RabbitError::Msg(e.to_string()))?;
        Ok(())
    }

    async fn bind(
        &self,
        queue: &str,
        exchange: &str,
        routing_key: &str,
    ) -> Result<(), RabbitError> {
        let ch = self.ch().await?;
        ch.queue_bind(
            queue,
            exchange,
            routing_key,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .map_err(|e| RabbitError::Msg(e.to_string()))?;
        Ok(())
    }

    async fn publish(
        &self,
        exchange: &Exchange,
        routing_key: &str,
        body: Bytes,
    ) -> Result<(), RabbitError> {
        let ch = self.ch().await?;
        ch.basic_publish(
            &exchange.name,
            routing_key,
            BasicPublishOptions::default(),
            &body,
            BasicProperties::default(),
        )
        .await
        .map_err(|e| RabbitError::Msg(e.to_string()))?
        .await
        .map_err(|e| RabbitError::Msg(e.to_string()))?;
        Ok(())
    }

    async fn consume_one(&self, queue: &str) -> Result<Option<Delivery>, RabbitError> {
        use futures_util::StreamExt;
        let ch = self.ch().await?;
        let mut consumer = ch
            .basic_consume(
                queue,
                "sova",
                BasicConsumeOptions {
                    no_ack: false,
                    ..BasicConsumeOptions::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| RabbitError::Msg(e.to_string()))?;

        let delivery = tokio::time::timeout(std::time::Duration::from_millis(50), consumer.next())
            .await
            .ok()
            .flatten()
            .transpose()
            .map_err(|e| RabbitError::Msg(e.to_string()))?;

        let Some(del) = delivery else {
            return Ok(None);
        };

        let tag = del.delivery_tag;
        let ch_ack = ch.clone();
        let ch_nack = ch.clone();
        Ok(Some(Delivery::new(
            del.exchange.to_string(),
            del.routing_key.to_string(),
            Bytes::from(del.data),
            move |_multiple| {
                let ch = ch_ack;
                async move {
                    ch.basic_ack(tag, BasicAckOptions::default())
                        .await
                        .map_err(|e| RabbitError::Msg(e.to_string()))
                }
            },
            move |requeue| {
                let ch = ch_nack;
                async move {
                    ch.basic_nack(
                        tag,
                        BasicNackOptions {
                            requeue,
                            ..BasicNackOptions::default()
                        },
                    )
                    .await
                    .map_err(|e| RabbitError::Msg(e.to_string()))
                }
            },
        )))
    }
}
