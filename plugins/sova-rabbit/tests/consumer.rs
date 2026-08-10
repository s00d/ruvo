//! RabbitConsumer BackgroundService + startup validation.

use bytes::Bytes;
use sova_core::extend::Bind;
use sova_core::{App, Error};
use sova_rabbit::{Broker, Exchange, FakeBroker, QueueOpts, Rabbit, RabbitConsumer};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn empty_url_fails_startup() {
    let mut app = App::new();
    app.install(Rabbit::from_env().url(""));
    let err = match app.run_startup().await {
        Err(e) => e,
        Ok(_) => panic!("expected startup error"),
    };
    assert!(matches!(err, Error::Internal(_)));
    assert!(err.to_string().contains("rabbit url is empty"));
}

#[tokio::test]
async fn background_consumer_acks() {
    let processed = Arc::new(AtomicUsize::new(0));
    let fake = FakeBroker::new();
    fake.declare_exchange(&Exchange::direct("main"))
        .await
        .unwrap();
    fake.declare_queue("jobs", &QueueOpts::durable())
        .await
        .unwrap();
    fake.bind("jobs", "main", "k").await.unwrap();

    let mut app = App::new();
    app.install(Rabbit::fake(fake.clone()));
    let counter = Arc::clone(&processed);
    RabbitConsumer::new("jobs", move |msg| {
        let counter = Arc::clone(&counter);
        async move {
            assert_eq!(msg.body.as_ref(), b"job");
            msg.ack().await?;
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    })
    .poll_interval(std::time::Duration::from_millis(10))
    .install(&mut app);

    fake.publish(&Exchange::direct("main"), "k", Bytes::from_static(b"job"))
        .await
        .unwrap();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        app.bind(Bind::Listener(listener))
            .shutdown(async move {
                let _ = rx.await;
            })
            .serve()
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert_eq!(processed.load(Ordering::SeqCst), 1);
    let _ = tx.send(());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server)
        .await
        .expect("server stop")
        .expect("join");
}
