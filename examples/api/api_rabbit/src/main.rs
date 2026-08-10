//! RabbitMQ demo — FakeBroker by default; set AMQP_URL for live broker.

use bytes::Bytes;
use sova::{
    App, Exchange, FakeBroker, Json, QueueOpts, Rabbit, RabbitConsumer, RabbitExt, Request, Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();

    if std::env::var("AMQP_URL")
        .or_else(|_| std::env::var("RABBITMQ_URL"))
        .is_ok()
    {
        app.install(Rabbit::from_env());
    } else {
        let fake = FakeBroker::new();
        app.install(Rabbit::fake(fake));
        RabbitConsumer::new("jobs", |msg| async move {
            eprintln!(
                "consumed {} from {}: {:?}",
                msg.routing_key,
                msg.exchange,
                String::from_utf8_lossy(&msg.body)
            );
            msg.ack().await?;
            Ok::<(), sova::RabbitError>(())
        })
        .install(&mut app);
    }

    app.post("/publish", |req: Request| async move {
        let ex = Exchange::topic("events");
        req.rabbit().declare_exchange(&ex).await?;
        req.rabbit()
            .declare_queue("jobs", &QueueOpts::durable())
            .await?;
        req.rabbit().bind("jobs", "events", "demo.*").await?;
        req.rabbit()
            .publish(&ex, "demo.ping", Bytes::from_static(b"{\"ok\":true}"))
            .await?;
        Ok::<_, sova::Error>(Json(serde_json::json!({ "published": true })))
    });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    eprintln!("api_rabbit listening on http://127.0.0.1:{port}");
    app.listen(port).await
}
