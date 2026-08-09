//! RedisPool without a live server (url / get / clear).

use sova_redis::RedisPool;

#[tokio::test]
async fn url_set_get_and_errors() {
    let pool = RedisPool::new();
    assert!(pool.url().is_err());
    assert!(pool.get().is_err());

    pool.set_url("redis://127.0.0.1:6379");
    assert_eq!(pool.url().unwrap(), "redis://127.0.0.1:6379");
    assert!(pool.get().is_err());

    pool.clear();
    assert!(pool.get().is_err());
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
