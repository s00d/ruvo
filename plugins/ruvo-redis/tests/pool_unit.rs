//! RedisPool without a live server (url / get / clear).

use ruvo_redis::RedisPool;

#[tokio::test]
async fn url_set_get_and_errors() {
    let pool = RedisPool::new();
    assert!(pool.url().await.is_err());
    assert!(pool.get().await.is_err());

    pool.set_url("redis://127.0.0.1:6379").await;
    assert_eq!(pool.url().await.unwrap(), "redis://127.0.0.1:6379");
    assert!(pool.get().await.is_err());

    pool.clear().await;
    assert!(pool.get().await.is_err());
}

#[tokio::test]
async fn connect_bad_url_is_err() {
    let err = match RedisPool::connect("not-a-redis-url").await {
        Ok(_) => panic!("expected connect error"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(!msg.is_empty());
}
