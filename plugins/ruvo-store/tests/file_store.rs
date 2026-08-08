//! FileStore persistence, TTL, fsync, snapshot compaction.

#![cfg(feature = "file")]

use bytes::Bytes;
use ruvo_store::{Durability, FileStore, KvStore};
use std::time::Duration;

#[tokio::test]
async fn set_get_remove_incr_clear_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileStore::open(dir.path()).await.unwrap();

    store.set("a", Bytes::from_static(b"1"), None).await;
    assert_eq!(store.get("a").await.as_deref(), Some(b"1".as_slice()));

    let n = store.incr("cnt", 2, None).await;
    assert_eq!(n, 2);
    assert_eq!(store.incr("cnt", 3, None).await, 5);

    store.set("pref:x", Bytes::from_static(b"x"), None).await;
    store.set("pref:y", Bytes::from_static(b"y"), None).await;
    store.set("other", Bytes::from_static(b"z"), None).await;
    assert_eq!(store.clear_prefix("pref:").await, 2);
    assert!(store.get("pref:x").await.is_none());
    assert_eq!(store.get("other").await.as_deref(), Some(b"z".as_slice()));

    store.remove("other").await;
    assert!(store.get("other").await.is_none());
}

#[tokio::test]
async fn ttl_expires_and_survives_reopen_via_log() {
    let dir = tempfile::tempdir().unwrap();
    {
        let store = FileStore::open(dir.path()).await.unwrap();
        store
            .set("live", Bytes::from_static(b"ok"), None)
            .await;
        store
            .set(
                "soon",
                Bytes::from_static(b"x"),
                Some(Duration::from_millis(40)),
            )
            .await;
        tokio::time::sleep(Duration::from_millis(70)).await;
        assert!(store.get("soon").await.is_none());
        assert_eq!(store.get("live").await.as_deref(), Some(b"ok".as_slice()));
    }
    let store2 = FileStore::open(dir.path()).await.unwrap();
    assert_eq!(store2.get("live").await.as_deref(), Some(b"ok".as_slice()));
}

#[tokio::test]
async fn fsync_durability_and_snapshot_compaction() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileStore::open_with(dir.path(), Durability::Fsync)
        .await
        .unwrap()
        .durability(Durability::Fsync);

    for i in 0..260 {
        let key = format!("k{i}");
        store
            .set(&key, Bytes::from(format!("v{i}")), None)
            .await;
    }
    assert_eq!(
        store.get("k0").await.as_deref(),
        Some(b"v0".as_slice())
    );
    drop(store);

    assert!(dir.path().join("snapshot.bin").exists());
    let reopened = FileStore::open(dir.path()).await.unwrap();
    assert_eq!(
        reopened.get("k259").await.as_deref(),
        Some(b"v259".as_slice())
    );
}
