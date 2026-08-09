**When:** persist task queue (memory / file / sql / redis).

**Does:**
- `TaskStore` backends for `tasks`
- Feature-gated drivers

### Example

```rust
app.install(Tasks::new().store(RedisTaskStore::from_pool(pool)));
```
