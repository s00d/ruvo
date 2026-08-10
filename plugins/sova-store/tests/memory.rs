//! MemoryStore put / get / ttl.

use bytes::Bytes;
use sova_store::{KvStore, MemoryStore};
use std::time::Duration;

#[tokio::test]
async fn put_get() {
    let store = MemoryStore::new();
    store.set("k", Bytes::from_static(b"v"), None).await;
    assert_eq!(store.get("k").await.as_deref(), Some(b"v".as_slice()));
    store.remove("k").await;
    assert!(store.get("k").await.is_none());
}

#[tokio::test]
async fn ttl_expires() {
    let store = MemoryStore::new();
    store
        .set(
            "t",
            Bytes::from_static(b"1"),
            Some(Duration::from_millis(40)),
        )
        .await;
    assert_eq!(store.get("t").await.as_deref(), Some(b"1".as_slice()));
    tokio::time::sleep(Duration::from_millis(70)).await;
    assert!(store.get("t").await.is_none());
}

#[tokio::test]
async fn app_store_memory_put_get() {
    use sova_store::AppStore;
    let app_store = AppStore::memory();
    app_store
        .inner
        .set("a", Bytes::from_static(b"b"), None)
        .await;
    assert_eq!(
        app_store.inner.get("a").await.as_deref(),
        Some(b"b".as_slice())
    );
}
