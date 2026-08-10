//! In-memory broker for tests (exchanges, bindings, ack/nack, DLQ).

use crate::broker::{Broker, Delivery, Exchange, ExchangeKind, QueueOpts};
use crate::error::RabbitError;
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Pending {
    exchange: String,
    routing_key: String,
    body: Bytes,
    #[allow(dead_code)]
    redelivered: bool,
}

#[derive(Default)]
struct Inner {
    exchanges: HashMap<String, ExchangeKind>,
    /// queue -> messages
    queues: HashMap<String, VecDeque<Pending>>,
    queue_opts: HashMap<String, QueueOpts>,
    /// (exchange, routing_key pattern) -> queues
    bindings: Vec<(String, String, String)>,
}

#[derive(Clone, Default)]
pub struct FakeBroker {
    inner: Arc<Mutex<Inner>>,
}

impl FakeBroker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn queue_len(&self, queue: &str) -> usize {
        self.inner
            .lock()
            .unwrap()
            .queues
            .get(queue)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    fn route_locked(inner: &mut Inner, exchange: &str, routing_key: &str, body: Bytes) {
        let kind = inner
            .exchanges
            .get(exchange)
            .copied()
            .unwrap_or(ExchangeKind::Direct);
        let targets: Vec<String> = inner
            .bindings
            .iter()
            .filter(|(ex, key, _)| {
                if ex != exchange {
                    return false;
                }
                match kind {
                    ExchangeKind::Fanout => true,
                    ExchangeKind::Direct => key == routing_key,
                    ExchangeKind::Topic => topic_match(key, routing_key),
                    ExchangeKind::Headers => key == routing_key,
                }
            })
            .map(|(_, _, q)| q.clone())
            .collect();

        for q in targets {
            inner.queues.entry(q).or_default().push_back(Pending {
                exchange: exchange.to_string(),
                routing_key: routing_key.to_string(),
                body: body.clone(),
                redelivered: false,
            });
        }
    }
}

fn topic_match(pattern: &str, key: &str) -> bool {
    let pat: Vec<&str> = pattern.split('.').collect();
    let key: Vec<&str> = key.split('.').collect();
    let mut i = 0;
    let mut j = 0;
    while i < pat.len() && j < key.len() {
        match pat[i] {
            "#" => return true,
            "*" => {
                i += 1;
                j += 1;
            }
            p if p == key[j] => {
                i += 1;
                j += 1;
            }
            _ => return false,
        }
    }
    (i == pat.len() && j == key.len()) || (i == pat.len() - 1 && pat[i] == "#")
}

#[async_trait]
impl Broker for FakeBroker {
    async fn declare_exchange(&self, exchange: &Exchange) -> Result<(), RabbitError> {
        self.inner
            .lock()
            .unwrap()
            .exchanges
            .insert(exchange.name.clone(), exchange.kind);
        Ok(())
    }

    async fn declare_queue(&self, name: &str, opts: &QueueOpts) -> Result<(), RabbitError> {
        let mut g = self.inner.lock().unwrap();
        g.queues.entry(name.to_string()).or_default();
        g.queue_opts.insert(name.to_string(), opts.clone());
        Ok(())
    }

    async fn bind(
        &self,
        queue: &str,
        exchange: &str,
        routing_key: &str,
    ) -> Result<(), RabbitError> {
        self.inner.lock().unwrap().bindings.push((
            exchange.to_string(),
            routing_key.to_string(),
            queue.to_string(),
        ));
        Ok(())
    }

    async fn publish(
        &self,
        exchange: &Exchange,
        routing_key: &str,
        body: Bytes,
    ) -> Result<(), RabbitError> {
        let mut g = self.inner.lock().unwrap();
        if !g.exchanges.contains_key(&exchange.name) {
            g.exchanges.insert(exchange.name.clone(), exchange.kind);
        }
        FakeBroker::route_locked(&mut g, &exchange.name, routing_key, body);
        Ok(())
    }

    async fn consume_one(&self, queue: &str) -> Result<Option<Delivery>, RabbitError> {
        let pending = {
            let mut g = self.inner.lock().unwrap();
            g.queues.get_mut(queue).and_then(|q| q.pop_front())
        };
        let Some(msg) = pending else {
            return Ok(None);
        };

        let broker = self.clone();
        let queue = queue.to_string();
        let msg_ack = msg.clone();
        let msg_nack = msg.clone();

        Ok(Some(Delivery::new(
            msg.exchange,
            msg.routing_key,
            msg.body,
            move |_multiple| {
                let _ = msg_ack;
                async move { Ok(()) }
            },
            move |requeue| {
                let broker = broker.clone();
                let queue = queue.clone();
                async move {
                    if requeue {
                        let mut g = broker.inner.lock().unwrap();
                        g.queues.entry(queue).or_default().push_front(Pending {
                            exchange: msg_nack.exchange,
                            routing_key: msg_nack.routing_key,
                            body: msg_nack.body,
                            redelivered: true,
                        });
                    } else {
                        // dead-letter if configured
                        let mut g = broker.inner.lock().unwrap();
                        if let Some(opts) = g.queue_opts.get(&queue).cloned() {
                            if let Some(dlx) = opts.dead_letter_exchange {
                                let rk = opts
                                    .dead_letter_routing_key
                                    .unwrap_or_else(|| queue.clone());
                                FakeBroker::route_locked(
                                    &mut g,
                                    &dlx,
                                    &rk,
                                    msg_nack.body.clone(),
                                );
                            }
                        }
                    }
                    Ok(())
                }
            },
        )))
    }
}
