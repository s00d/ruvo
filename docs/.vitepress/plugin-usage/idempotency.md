```rust
app.install(SharedStore::memory());
app.install(Idempotency::from_app(&app).ttl(std::time::Duration::from_secs(3600)));
```

Or `Idempotency::from_store(kv)` with an explicit [`KvStore`].
