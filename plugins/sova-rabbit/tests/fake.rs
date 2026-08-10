use bytes::Bytes;
use sova_core::{App, Request, ResponseAssert, TestClient};
use sova_rabbit::{Broker, Exchange, FakeBroker, QueueOpts, Rabbit, RabbitExt};

#[tokio::test]
async fn fake_publish_consume_ack() {
    let fake = FakeBroker::new();
    let mut app = App::new();
    app.install(Rabbit::fake(fake.clone()));
    app.post("/send", |req: Request| async move {
        let ex = Exchange::topic("events");
        req.rabbit().declare_exchange(&ex).await.unwrap();
        req.rabbit()
            .declare_queue("jobs", &QueueOpts::durable())
            .await
            .unwrap();
        req.rabbit().bind("jobs", "events", "user.*").await.unwrap();
        req.rabbit()
            .publish(&ex, "user.created", Bytes::from_static(b"{\"id\":1}"))
            .await
            .unwrap();
        sova_core::Json(serde_json::json!({ "ok": true }))
    });

    let c = TestClient::new(app).unwrap();
    c.post("/send").await.assert_status(200);

    let msg = fake.consume_one("jobs").await.unwrap().expect("msg");
    assert_eq!(msg.routing_key, "user.created");
    assert_eq!(msg.body.as_ref(), b"{\"id\":1}");
    msg.ack().await.unwrap();
    assert_eq!(fake.queue_len("jobs"), 0);
}

#[tokio::test]
async fn fake_nack_to_dlq() {
    let fake = FakeBroker::new();
    fake.declare_exchange(&Exchange::direct("dlx"))
        .await
        .unwrap();
    fake.declare_queue("dlq", &QueueOpts::durable())
        .await
        .unwrap();
    fake.bind("dlq", "dlx", "jobs").await.unwrap();
    fake.declare_queue("jobs", &QueueOpts::durable().with_dlq("dlx", "jobs"))
        .await
        .unwrap();
    fake.declare_exchange(&Exchange::direct("main"))
        .await
        .unwrap();
    fake.bind("jobs", "main", "k").await.unwrap();
    fake.publish(&Exchange::direct("main"), "k", Bytes::from_static(b"x"))
        .await
        .unwrap();

    let msg = fake.consume_one("jobs").await.unwrap().unwrap();
    msg.nack(false).await.unwrap();
    assert_eq!(fake.queue_len("dlq"), 1);
}
