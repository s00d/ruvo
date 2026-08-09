//! Pub/Sub + list queue smoke (needs `REDIS_URL`).

use sova_redis::RedisPool;
use std::time::Duration;

async fn pool() -> Option<RedisPool> {
    let url = std::env::var("REDIS_URL").ok()?;
    let conn = RedisPool::connect(&url).await.ok()?;
    let pool = RedisPool::new();
    pool.set_url(url);
    pool.set(conn);
    Some(pool)
}

#[tokio::test]
async fn publish_subscribe_roundtrip() {
    let Some(pool) = pool().await else {
        eprintln!("skip publish_subscribe_roundtrip: set REDIS_URL");
        return;
    };
    let channel = format!(
        "sova_test_ch_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );

    let mut sub = pool.subscribe([&channel]).await.unwrap();
    // Give subscription a moment to register.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let n = pool.publish(&channel, b"hello").await.unwrap();
    assert!(n >= 1, "expected at least one subscriber, got {n}");

    let msg = tokio::time::timeout(Duration::from_secs(2), sub.next())
        .await
        .expect("timeout waiting for pubsub message")
        .expect("stream ended");
    assert_eq!(msg.channel, channel);
    assert_eq!(msg.payload_str(), Some("hello"));
}

#[tokio::test]
async fn enqueue_dequeue_roundtrip() {
    let Some(pool) = pool().await else {
        eprintln!("skip enqueue_dequeue_roundtrip: set REDIS_URL");
        return;
    };
    let queue = format!(
        "sova_test_q_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );

    assert_eq!(pool.dequeue(&queue).await.unwrap(), None);
    pool.enqueue(&queue, b"one").await.unwrap();
    pool.enqueue(&queue, b"two").await.unwrap();
    // LPUSH + RPOP => FIFO: one then two
    assert_eq!(pool.dequeue(&queue).await.unwrap().as_deref(), Some(b"one".as_slice()));
    assert_eq!(pool.dequeue(&queue).await.unwrap().as_deref(), Some(b"two".as_slice()));
    assert_eq!(pool.dequeue(&queue).await.unwrap(), None);
}

#[tokio::test]
async fn dequeue_wait_timeout() {
    let Some(pool) = pool().await else {
        eprintln!("skip dequeue_wait_timeout: set REDIS_URL");
        return;
    };
    let queue = format!(
        "sova_test_bq_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let got = pool
        .dequeue_wait(&queue, Duration::from_millis(200))
        .await
        .unwrap();
    assert!(got.is_none());
}
